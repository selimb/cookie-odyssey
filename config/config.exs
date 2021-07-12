# This file is responsible for configuring your application
# and its dependencies with the aid of the Mix.Config module.
#
# This configuration file is loaded before any dependency and
# is restricted to this project.

# General application configuration
use Mix.Config

config :cookie_odyssey,
  ecto_repos: [CookieOdyssey.Repo]

# Configures the endpoint
config :cookie_odyssey, CookieOdysseyWeb.Endpoint,
  url: [host: "localhost"],
  secret_key_base: "vRfC+Xx2Q8IDUEMohTTmrRyNHDLZWohcpaFZE3Yl5uWf20oAPLsrKUnMCLeb/sWg",
  render_errors: [view: CookieOdysseyWeb.ErrorView, accepts: ~w(html json), layout: false],
  pubsub_server: CookieOdyssey.PubSub,
  live_view: [signing_salt: "vbId/CT7"]

# Configures Elixir's Logger
config :logger, :console,
  format: "$time $metadata[$level] $message\n",
  metadata: [:request_id]

# Use Jason for JSON parsing in Phoenix
config :phoenix, :json_library, Jason

# Import environment specific config. This must remain at the bottom
# of this file so it overrides the configuration defined above.
import_config "#{Mix.env()}.exs"
