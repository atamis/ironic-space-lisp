defmodule ISL.Parser do
  import NimbleParsec

  ws = ignore(repeat(utf8_char([?\s, ?\t, ?\n])))
  defparsec :ws, ws

  keyword_start = utf8_char([
    ?a..?z,
    ?A..?Z,
    ?-, ?!, ??, ?*, ?+, ?/, ?$, ?<, ?>, ?., ?=
  ])

  defparsec :keyword_start, keyword_start

  keyword = keyword_start |> repeat(choice([keyword_start, utf8_char([?0..?9])])) |> reduce({:keyword_convert, []})
  defparsec :keyword, keyword


  number = integer(min: 1)
  defparsec :number, number

  bool = choice([string("#t"), string("#f")]) |> map({:parse_bool, []})
  defparsec :bool, bool

  # empty list implicit
  list = ignore(string("(")) |> parsec(:exprs) |> ignore(string(")"))

  expr = ignore(ws) |> choice([number, list, bool, keyword]) |> ignore(ws)
  defparsec :expr, expr

  exprs = repeat(expr) |> reduce({:ident, []})
  defparsec :exprs, exprs

  def read(parser, string) do
    {:ok, ans, "", _, _, _} = :erlang.apply(__MODULE__, parser, [string])
    {:ok, ans}
  end

  def keyword_convert(charlist) when is_list(charlist) do
    to_string(charlist)
  end

  def ident(x), do: x

  def parse_bool("#t"), do: true
  def parse_bool("#f"), do: false
end
