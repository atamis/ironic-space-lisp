defprotocol ISL.Applicable do
  #@spec apply(list(isl_value())) :: isl_value()
  def apply(func, args, env)
end

defmodule ISL.Lambda do
  @enforce_keys [:args, :body]
  defstruct [:args, :body]
end

defimpl ISL.Applicable, for: ISL.Lambda do
  def apply(%ISL.Lambda{args: names, body: body}, args, env) when length(names) == length(args) do
    # TODO: error on non-keyword names
    benv = Enum.zip(names, args) |> Enum.into(env)
    {val, _} = ISL.eval(body, benv)
    val
  end
end

defimpl ISL.Applicable, for: Function do
  def apply(fun, args, _) do
    :erlang.apply(fun, args)
  end
end

defmodule ISL do
  @moduledoc """
  Documentation for Isl.
  """

  @type isl_value() ::
          String.t()
          | non_neg_integer()
          | list(isl_value())
          | boolean()

  @type isl_env() :: %{optional(String.t()) => isl_value()}

  @doc """
  Calculate the size of the ISL value.

  ## examples

      iex> ISL.size(4)
      1

      iex> ISL.size([1, 2, 3])
      3

      iex> ISL.size("test")
      4
  """
  @spec size(isl_value()) :: integer()
  def size(l) when is_list(l), do: l |> Enum.map(&size/1) |> Enum.reduce(&+/2)
  def size(b) when is_binary(b), do: String.length(b)
  def size(_), do: 1

  @spec eval(isl_value()) :: {isl_value(), isl_env()}
  def eval(e), do: eval(e, %{})

  @spec eval(isl_value(), isl_env()) :: {isl_value(), isl_env()}
  def eval(["let", bindings, then], env) do
    subenv =
      bindings
      |> bindings_map()

    new_env = Map.merge(env, subenv)
    {val, _} = eval(then, new_env)
    {val, env}
  end

  def eval(["def", name, value], env) do
    {val, venv} = eval(value, env)
    {val, Map.put(venv, name, val)}
  end

  def eval(["if", pred, then, els], env) do
    {v, nenv} = eval(pred, env)

    if v do
      eval(then, nenv)
    else
      eval(els, nenv)
    end
  end

  def eval(["do" | exprs], env) when length(exprs) > 0 do
    Enum.reduce(exprs, {nil, env}, fn expr, {_, env} ->
      eval(expr, env)
    end)
  end

  def eval(["lambda", args, body], env) when is_list(args) do
    {%ISL.Lambda{args: args, body: body}, env}
  end

  def eval([_ | args] = exprs, env) when is_list(args) do
    {[fun | args], subenv} = Enum.map_reduce(exprs, env, fn expr, senv ->
      eval(expr, senv)
    end)

    {ISL.Applicable.apply(fun, args, subenv), subenv}
  end

  def eval(e, env) when is_binary(e) and is_map(env) do
    {Map.fetch!(env, e), env}
  end

  def eval(e, env) when is_boolean(e) or is_integer(e), do: {e, env}

  @doc """
  Converts an idiomatic list of bindings to a map.
  """
  def bindings_map(bindings) do
    bindings
    |> Enum.chunk_every(2)
    |> Enum.map(&convert_binding/1)
    |> Enum.into(%{})
  end

  defp convert_binding([name, value]) when is_binary(name) do
    {name, value}
  end

  @doc """
  Hello world.

  ## Examples

      iex> ISL.hello()
      :world

  """
  def hello do
    :world
  end
end
