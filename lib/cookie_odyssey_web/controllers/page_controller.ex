defmodule CookieOdysseyWeb.PageController do
  use CookieOdysseyWeb, :controller

  def index(conn, _params) do
    render(conn, "index.html")
  end
end
