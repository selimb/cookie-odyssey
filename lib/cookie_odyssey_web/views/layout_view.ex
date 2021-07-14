defmodule CookieOdysseyWeb.LayoutView do
  use CookieOdysseyWeb, :view

  def title() do
    "Cookie Odyssey"
  end

  # Inpsired by https://elixirforum.com/t/nest-eex-templates/18462
  def nav_item(assigns, do: children) do
    render(
      "nav_item.html",
      put_in(assigns[:children], children)
    )
  end
end
