//! Cards are what a Journal or Scrapbook page lists, and what Previous/Next
//! steps through.
//! A Day card is the visible Entries of one date.
//! A Getaway card is the visible Entries of one Getaway, which only a
//! Scrapbook has, see [journal-no-getaway].
use itertools::Itertools as _;
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, FromQueryResult, JoinType, QueryFilter,
    QuerySelect as _, RelationTrait as _,
};
use serde::Serialize;

use crate::{
    journal::{
        helpers::get_journal_day_number_for_date,
        routes::{JournalDayGetPath, JournalGetawayGetPath},
    },
    AuthSession, Route,
};
use entities::{prelude::*, *};

#[derive(Serialize, Clone, Debug)]
pub struct EntrySlim {
    pub id: i32,
    pub title: String,
    pub draft: bool,
}

/// The columns of an Entry that cards need.
#[derive(FromQueryResult, Debug)]
struct EntryRow {
    id: i32,
    title: String,
    date: chrono::NaiveDate,
    time: chrono::NaiveTime,
    draft: bool,
    getaway_id: Option<i32>,
    getaway_name: Option<String>,
}

#[derive(Debug)]
pub enum CardKind {
    Day,
    Getaway { id: i32, name: String },
}

#[derive(Debug)]
pub struct Card {
    pub kind: CardKind,
    /// The earliest date of the visible Entries.
    pub start: chrono::NaiveDate,
    /// The latest date of the visible Entries.
    pub end: chrono::NaiveDate,
    /// In trip order.
    pub entries: Vec<EntrySlim>,
}

/// Orders cards from earliest to latest.
/// When a Getaway and a Day start on the same date, the Getaway is later, so
/// that it is listed first when the list is reversed, see [cards-newest-first].
#[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct CardKey {
    start: chrono::NaiveDate,
    is_getaway: bool,
    getaway_id: i32,
}

impl CardKey {
    pub fn day(date: chrono::NaiveDate) -> Self {
        Self {
            start: date,
            is_getaway: false,
            getaway_id: 0,
        }
    }

    pub fn getaway(start: chrono::NaiveDate, getaway_id: i32) -> Self {
        Self {
            start,
            is_getaway: true,
            getaway_id,
        }
    }
}

impl Card {
    pub fn key(&self) -> CardKey {
        match self.kind {
            CardKind::Day => CardKey::day(self.start),
            CardKind::Getaway { id, .. } => CardKey::getaway(self.start, id),
        }
    }
}

/// Returns the cards of the Journal that the User can see, from earliest to
/// latest.
/// In a Journal, they are all Day cards.
pub async fn query_cards(
    journal: &journal::Model,
    db: &DatabaseConnection,
    auth: &AuthSession,
) -> Result<Vec<Card>, anyhow::Error> {
    let mut q = JournalEntry::find()
        .select_only()
        .columns([
            journal_entry::Column::Id,
            journal_entry::Column::Title,
            journal_entry::Column::Date,
            journal_entry::Column::Time,
            journal_entry::Column::Draft,
            journal_entry::Column::GetawayId,
        ])
        .join(
            JoinType::LeftJoin,
            journal_entry::Relation::JournalGetaway.def(),
        )
        .column_as(journal_getaway::Column::Name, "getaway_name")
        .filter(journal_entry::Column::JournalId.eq(journal.id));
    q = auth.backend.filter_journal_entries(auth, q).await?;
    let entries = q.into_model::<EntryRow>().all(db).await?;

    let mut cards: Vec<Card> = Vec::new();
    let entries = entries
        .into_iter()
        .sorted_by_key(|e| (e.date, e.time, e.id));
    let (in_getaway, loose): (Vec<_>, Vec<_>) = entries.partition(|e| e.getaway_id.is_some());

    for (getaway_id, chunk) in &in_getaway
        .into_iter()
        .into_group_map_by(|e| e.getaway_id.expect("Partitioned on Some"))
    {
        let first = chunk.first().expect("Groups are not empty");
        let end = chunk.last().expect("Groups are not empty").date;
        // The join always finds the Getaway, see [entry-getaway-fk].
        let name = first.getaway_name.clone().expect("Getaway exists");
        cards.push(Card {
            kind: CardKind::Getaway {
                id: *getaway_id,
                name,
            },
            start: first.date,
            end,
            entries: chunk.iter().map(entry_slim).collect(),
        });
    }
    for (date, chunk) in &loose.into_iter().chunk_by(|e| e.date) {
        cards.push(Card {
            kind: CardKind::Day,
            start: date,
            end: date,
            entries: chunk.map(|e| entry_slim(&e)).collect(),
        });
    }

    cards.sort_by_key(Card::key);
    Ok(cards)
}

fn entry_slim(e: &EntryRow) -> EntrySlim {
    EntrySlim {
        id: e.id,
        title: e.title.clone(),
        draft: e.draft,
    }
}

/// Returns the cards just before and after `key`, for Previous/Next.
/// `key` need not be the key of a card of the Journal.
pub async fn query_neighbours(
    journal: &journal::Model,
    key: &CardKey,
    db: &DatabaseConnection,
    auth: &AuthSession,
) -> Result<(Option<CardView>, Option<CardView>), anyhow::Error> {
    let cards = query_cards(journal, db, auth).await?;
    let prev = cards.iter().rev().find(|c| c.key() < *key);
    let next = cards.iter().find(|c| c.key() > *key);
    Ok((
        prev.map(|card| CardView::new(card, journal)),
        next.map(|card| CardView::new(card, journal)),
    ))
}

/// A card as rendered by templates.
#[derive(Serialize, Debug)]
pub struct CardView {
    pub href: String,
    /// See [`day_number`].
    /// Only for Day cards.
    pub day_number: Option<i64>,
    /// Only for Getaway cards.
    pub getaway_name: Option<String>,
    pub start: chrono::NaiveDateTime,
    pub end: chrono::NaiveDateTime,
    pub entries: Vec<EntrySlim>,
}

impl CardView {
    pub fn new(card: &Card, journal: &journal::Model) -> Self {
        let (href, day_number, getaway_name) = match &card.kind {
            CardKind::Day => {
                let href = Route::JournalDayGet(Some(&JournalDayGetPath {
                    slug: journal.slug.clone(),
                    date: card.start,
                }))
                .as_path();
                (
                    href,
                    get_journal_day_number_for_date(journal, card.start),
                    None,
                )
            }
            CardKind::Getaway { id, name } => {
                let href = Route::JournalGetawayGet(Some(&JournalGetawayGetPath {
                    slug: journal.slug.clone(),
                    getaway_id: *id,
                }))
                .as_path();
                (href, None, Some(name.clone()))
            }
        };
        Self {
            href: href.into_owned(),
            day_number,
            getaway_name,
            // See [date-as-datetime].
            start: chrono::NaiveDateTime::new(card.start, Default::default()),
            end: chrono::NaiveDateTime::new(card.end, Default::default()),
            entries: card.entries.clone(),
        }
    }
}
