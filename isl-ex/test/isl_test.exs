defmodule ISLTest do
  use ExUnit.Case
  doctest ISL

  import ISL

  test "value sizing" do
    assert size(1) == 1
    assert size(true) == 1
    assert size("test") == 4
    assert size([1, 2, 3]) == 3
    assert size(["test", "asdf"]) == 8
  end

  def noenv(v) do
    {v, %{}}
  end

  test "evaluating values" do
    assert eval(1) == noenv(1)
    assert eval(true) == noenv(true)
  end

  test "evaluating if expressions" do
    assert eval(["if", true, 1, 0]) == noenv(1)
    assert eval(["if", false, 1, 0]) == noenv(0)
  end

  test "environment lookup" do
    assert eval("test", %{"test" => 3}) == {3, %{"test" => 3}}

    assert_raise KeyError, fn ->
      eval("test")
    end

    assert_raise KeyError, fn ->
      eval("test", %{"asdf" => 3})
    end
  end

  test "let bindings" do
    assert eval(["let", ["test", 1], "test"]) == noenv(1)
  end

  test "def" do
    assert eval(["def", "test", 4]) == {4, %{"test" => 4}}
    assert eval(["def", "test", ["def", "test2", 4]]) == {4, %{"test" => 4, "test2" => 4}}
  end

  test "do" do
    # TODO: expected FunctionClauseError, but this is interpreted as an application
    assert_raise KeyError, fn ->
      eval(["do"])
    end

    assert eval(["do", 1]) == noenv(1)
    assert eval(["do", ["def", "test", 2], "test"]) == {2, %{"test" => 2}}
    assert eval(["let", [], ["do", ["def", "test", 2], "test"]]) == noenv(2)
  end

  test "lambda creation" do
    assert eval(["lambda", ["test"], 1]) == noenv(%ISL.Lambda{args: ["test"], body: 1})
  end

  test "lambda application" do
    assert eval([["lambda", ["test"], "test"], 1]) == noenv(1)
    assert eval([["lambda", [], 1]]) == noenv(1)

    assert_raise FunctionClauseError, fn ->
      eval([["lambda", ["test"], "test"]])
    end
  end

  test "raw function application" do
    env = %{"+" => &+/2}
    assert eval(["+", 1, 1], env) == {2, env}
  end
end
