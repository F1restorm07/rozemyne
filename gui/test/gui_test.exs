defmodule GuiTest do
  use ExUnit.Case
  doctest Gui

  test "greets the world" do
    assert Gui.hello() == :world
  end
end
