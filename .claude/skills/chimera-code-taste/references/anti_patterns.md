# Anti-Patterns

## god_function
Function > 80 lines OR handling 5+ concerns. Symptom: docstring reads "X and also Y and Z". Fix: split by concern.

## magic_number
Raw numeric literals in logic without named constant. Fix: promote to constant or config.

## return_type_lie
Signature says `str`, reality returns `str | None` in edge cases. Fix the signature.

## test_through_main
Testing via `if __name__ == "__main__":` blocks. Fix: pytest.

## str_args_bag
Single `str` arg internally parsed as multi-field. Fix: Pydantic model.

## emoji_data
Emoji as data tokens in parse targets or stored fields. Fix: semantic names; rendering decides icons.