defmodule CookieOdyssey.Repo do
  use Ecto.Repo,
    otp_app: :cookie_odyssey,
    adapter: Ecto.Adapters.Postgres
end
