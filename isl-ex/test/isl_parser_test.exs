defmodule ISLParserTest do
  use ExUnit.Case, async: true
  doctest ISL.Parser

  import ISL.Parser

  test "whitespace" do
    assert {:ok, [], _, _, _, _} = ws("")
    assert {:ok, [], _, _, _, _} = ws(" ")
    assert {:ok, [], _, _, _, _} = ws("          ")
    assert {:ok, [], _, _, _, _} = ws("\t")
    assert {:ok, [], _, _, _, _} = ws("\t\t\t\t\t")
    assert {:ok, [], _, _, _, _} = ws("\n")
    assert {:ok, [], _, _, _, _} = ws("\n\n\n\n\n\n\n\n\n")
    assert {:ok, [], _, _, _, _} = ws(" \n\t \t \n")
    assert {:ok, [], <<"a", _::binary>>, _, _, _} = ws(" \n\t \t a \n")
  end

  test "number" do
    assert {:ok, [1], _, _, _, _} = number("1")
  end

  test "boolean" do
    assert {:ok, [true], _, _, _, _} = bool("#t")
    assert {:ok, [false], _, _, _, _} = bool("#f")
  end


  test "numbers" do
    assert {:ok, [1], _, _, _, _} = expr("    1")
    assert {:ok, [1], _, _, _, _} = expr("1     ")
    assert {:ok, [1], _, _, _, _} = expr("     1     ")
    assert {:ok, [[1, 2, 3]], _, _, _, _} = exprs("1 2 \n 3")
  end

  test "parenthesis" do
    assert {:ok, [[[1]]], _, _, _, _} = exprs("(1)")
    assert {:ok, [[[]]], _, _, _, _} = exprs("()")
    assert {:ok, [[[[[[[[[[[[1]]]]]]]]]]]], _, _, _, _} = exprs("((((((((((1))))))))))")
    assert {:ok, [[[1, 2]]], _, _, _, _} = exprs("(1 2)")
    assert {:ok, [[[[1], 1 ]]], _, _, _, _} = exprs("( ( 1 ) 1 )")
  end

  test "keywords" do
    assert {:ok, ["test"], _, _, _, _} = keyword("test")
    assert {:ok, ["t"], _, _, _, _} = keyword("t")
    assert {:ok, ["testasdfasdfasdf"], _, _, _, _} = keyword("testasdfasdfasdf")
    assert {:ok, ["t1"], _, _, _, _} = keyword("t1")
    assert {:ok, ["-!?*+?$<>.="], _, _, _, _} = keyword("-!?*+?$<>.=")
    # This looks nuts but it's how the rust parser works
    assert {:ok, [[1, "t"]], _, _, _, _} = exprs("1t")
    assert {:error, _, _, _, _, _} = keyword("1")
  end

end
