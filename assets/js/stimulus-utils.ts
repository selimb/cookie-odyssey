import type { Controller } from "@hotwired/stimulus";
import type { ValueDefinitionMap } from "@hotwired/stimulus/dist/types/core/value_properties";

type TargetDef = keyof HTMLElementTagNameMap | "element";
type TargetDefs = { [K: string]: TargetDef };

type InferTargetDef<T extends TargetDef> = T extends keyof HTMLElementTagNameMap
  ? HTMLElementTagNameMap[T]
  : HTMLElement;

export function defineTargets<T extends TargetDefs>(targetDefs: T) {
  return {
    targets: Object.keys(targetDefs),
    getTarget<K extends keyof T>(
      this: { [key: string]: any },
      name: K,
    ): InferTargetDef<T[K]> {
      const key = `${String(name)}Target`;
      const element = this[key];
      if (!element) throw new Error(`Missing target: ${String(name)}`);
      return element;
    },
  };
}

// This isn't publicly exported, and I don't like importing from `dist`.
type StimulusValueDefs = (typeof Controller)["values"];
type ValueDef = "string" | "number" | "boolean";
type ValueDefs = {
  [K: string]: ValueDef;
};

type InferValueDef<T extends ValueDef> = T extends "string"
  ? string
  : T extends "number"
    ? number
    : T extends "boolean"
      ? boolean
      : never;

const valueDefMap: Record<
  ValueDef,
  StimulusValueDefs[keyof StimulusValueDefs]
> = {
  string: String,
  number: Number,
  boolean: Boolean,
};

export function defineValues<T extends ValueDefs>(valueDefs: T) {
  const stimulusValueDefs: StimulusValueDefs = Object.fromEntries(
    Object.entries(valueDefs).map(([key, valueDef]) => {
      return [key, valueDefMap[valueDef]];
    }),
  );

  return {
    values: stimulusValueDefs,
    getValue<K extends keyof T>(this: { [key: string]: any }, name: K): T[K] {
      const existentialKey = `has${capitalize(String(name))}Value`;
      const exists = this[existentialKey] as boolean;
      if (!exists) throw new Error(`Missing value: ${String(name)}`);

      const getterKey = `${String(name)}Value`;
      const value = this[getterKey];
      if (!value) throw new Error(`Missing target: ${String(name)}`);
      return value;
    },
  };
}

function capitalize(key: string): string {
  return key.charAt(0).toUpperCase() + key.slice(1);
}
