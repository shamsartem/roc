#[macro_use]
extern crate pretty_assertions;
#[macro_use]
extern crate indoc;

extern crate bumpalo;
extern crate inkwell;
extern crate libc;
extern crate roc_gen;

#[macro_use]
mod helpers;

#[cfg(test)]
mod gen_dict {
    use roc_std::RocStr;

    #[test]
    fn dict_empty_len() {
        assert_evals_to!(
            indoc!(
                r#"
                    Dict.len Dict.empty
                "#
            ),
            0,
            usize
        );
    }

    #[test]
    fn dict_insert_empty() {
        assert_evals_to!(
            indoc!(
                r#"
                Dict.insert Dict.empty 42 32
                    |> Dict.len
                "#
            ),
            1,
            usize
        );
    }

    #[test]
    fn dict_empty_contains() {
        assert_evals_to!(
            indoc!(
                r#"
                empty : Dict I64 F64
                empty = Dict.empty

                Dict.contains empty 42
                "#
            ),
            false,
            bool
        );
    }

    #[test]
    fn dict_nonempty_contains() {
        assert_evals_to!(
            indoc!(
                r#"
                empty : Dict I64 F64
                empty = Dict.insert Dict.empty 42 3.14

                Dict.contains empty 42
                "#
            ),
            true,
            bool
        );
    }

    #[test]
    fn dict_empty_remove() {
        assert_evals_to!(
            indoc!(
                r#"
                empty : Dict I64 F64
                empty = Dict.empty

                empty
                    |> Dict.remove 42
                    |> Dict.len
                "#
            ),
            0,
            i64
        );
    }

    #[test]
    fn dict_nonempty_remove() {
        assert_evals_to!(
            indoc!(
                r#"
                empty : Dict I64 F64
                empty = Dict.insert Dict.empty 42 3.14

                empty
                    |> Dict.remove 42
                    |> Dict.len
                "#
            ),
            0,
            i64
        );
    }

    #[test]
    fn dict_nonempty_get() {
        assert_evals_to!(
            indoc!(
                r#"
                empty : Dict I64 F64
                empty = Dict.insert Dict.empty 42 3.14

                withDefault = \x, def ->
                    when  x is
                        Ok v -> v
                        Err _ -> def

                empty
                    |> Dict.insert 42 3.14
                    |> Dict.get 42
                    |> withDefault 0
                "#
            ),
            3.14,
            f64
        );

        assert_evals_to!(
            indoc!(
                r#"
                withDefault = \x, def ->
                    when  x is
                        Ok v -> v
                        Err _ -> def

                Dict.empty
                    |> Dict.insert 42 3.14
                    |> Dict.get 43
                    |> withDefault 0
                "#
            ),
            0.0,
            f64
        );
    }

    #[test]
    fn keys() {
        assert_evals_to!(
            indoc!(
                r#"
                myDict : Dict I64 I64
                myDict =
                    Dict.empty
                        |> Dict.insert 0 100
                        |> Dict.insert 1 100
                        |> Dict.insert 2 100


                Dict.keys myDict
                "#
            ),
            &[0, 1, 2],
            &[i64]
        );
    }

    #[test]
    fn values() {
        assert_evals_to!(
            indoc!(
                r#"
                myDict : Dict I64 I64
                myDict =
                    Dict.empty
                        |> Dict.insert 0 100
                        |> Dict.insert 1 200
                        |> Dict.insert 2 300


                Dict.values myDict
                "#
            ),
            &[100, 200, 300],
            &[i64]
        );
    }

    #[test]
    fn from_list_with_fold() {
        assert_evals_to!(
            indoc!(
                r#"
                myDict : Dict I64 I64
                myDict =
                    [1,2,3]
                        |> List.walk (\value, accum -> Dict.insert accum value value) Dict.empty

                Dict.values myDict
                "#
            ),
            &[2, 3, 1],
            &[i64]
        );

        assert_evals_to!(
            indoc!(
                r#"
                range : I64, I64, List I64-> List I64
                range = \low, high, accum ->
                    if low < high then
                        range (low + 1) high (List.append accum low)
                    else
                        accum

                myDict : Dict I64 I64
                myDict =
                    # 25 elements (8 + 16 + 1) is guaranteed to overflow/reallocate at least twice
                    range 0 25 []
                        |> List.walk (\value, accum -> Dict.insert accum value value) Dict.empty

                Dict.values myDict
                    |> List.len
                "#
            ),
            25,
            i64
        );
    }

    #[test]
    fn small_str_keys() {
        assert_evals_to!(
            indoc!(
                r#"
                myDict : Dict Str I64
                myDict =
                    Dict.empty
                        |> Dict.insert "a" 100
                        |> Dict.insert "b" 100
                        |> Dict.insert "c" 100


                Dict.keys myDict
                "#
            ),
            &[RocStr::from("c"), RocStr::from("a"), RocStr::from("b"),],
            &[RocStr]
        );
    }

    #[test]
    fn big_str_keys() {
        assert_evals_to!(
            indoc!(
                r#"
                myDict : Dict Str I64
                myDict =
                    Dict.empty
                        |> Dict.insert "Leverage agile frameworks to provide a robust" 100
                        |> Dict.insert "synopsis for high level overviews. Iterative approaches" 200
                        |> Dict.insert "to corporate strategy foster collaborative thinking to" 300


                Dict.keys myDict
                "#
            ),
            &[
                RocStr::from("Leverage agile frameworks to provide a robust"),
                RocStr::from("to corporate strategy foster collaborative thinking to"),
                RocStr::from("synopsis for high level overviews. Iterative approaches"),
            ],
            &[RocStr]
        );
    }

    #[test]
    fn big_str_values() {
        assert_evals_to!(
            indoc!(
                r#"
                myDict : Dict I64 Str
                myDict =
                    Dict.empty
                        |> Dict.insert 100 "Leverage agile frameworks to provide a robust"
                        |> Dict.insert 200 "synopsis for high level overviews. Iterative approaches"
                        |> Dict.insert 300 "to corporate strategy foster collaborative thinking to"

                Dict.values myDict
                "#
            ),
            &[
                RocStr::from("Leverage agile frameworks to provide a robust"),
                RocStr::from("to corporate strategy foster collaborative thinking to"),
                RocStr::from("synopsis for high level overviews. Iterative approaches"),
            ],
            &[RocStr]
        );
    }

    #[test]
    fn unit_values() {
        assert_evals_to!(
            indoc!(
                r#"
                myDict : Dict I64 {}
                myDict =
                    Dict.empty
                        |> Dict.insert 0 {}
                        |> Dict.insert 1 {}
                        |> Dict.insert 2 {}
                        |> Dict.insert 3 {}

                Dict.len myDict
                "#
            ),
            4,
            i64
        );
    }

    #[test]
    fn singleton() {
        assert_evals_to!(
            indoc!(
                r#"
                myDict : Dict I64 {}
                myDict =
                    Dict.singleton 0 {}

                Dict.len myDict
                "#
            ),
            1,
            i64
        );
    }

    #[test]
    fn union() {
        assert_evals_to!(
            indoc!(
                r#"
                myDict : Dict I64 {}
                myDict =
                    Dict.union (Dict.singleton 0 {}) (Dict.singleton 1 {})

                Dict.len myDict
                "#
            ),
            2,
            i64
        );
    }

    #[test]
    fn union_prefer_first() {
        assert_evals_to!(
            indoc!(
                r#"
                myDict : Dict I64 I64
                myDict =
                    Dict.union (Dict.singleton 0 100) (Dict.singleton 0 200)

                Dict.values myDict
                "#
            ),
            &[100],
            &[i64]
        );
    }

    #[test]
    fn intersection() {
        assert_evals_to!(
            indoc!(
                r#"
                dict1 : Dict I64 {}
                dict1 = 
                    Dict.empty
                        |> Dict.insert 1 {}
                        |> Dict.insert 2 {}
                        |> Dict.insert 3 {}
                        |> Dict.insert 4 {}
                        |> Dict.insert 5 {}

                dict2 : Dict I64 {}
                dict2 = 
                    Dict.empty
                        |> Dict.insert 0 {}
                        |> Dict.insert 2 {}
                        |> Dict.insert 4 {}

                Dict.intersection dict1 dict2 
                    |> Dict.len 
                "#
            ),
            2,
            i64
        );
    }

    #[test]
    fn intersection_prefer_first() {
        assert_evals_to!(
            indoc!(
                r#"
                dict1 : Dict I64 I64
                dict1 = 
                    Dict.empty
                        |> Dict.insert 1 1
                        |> Dict.insert 2 2
                        |> Dict.insert 3 3
                        |> Dict.insert 4 4
                        |> Dict.insert 5 5

                dict2 : Dict I64 I64
                dict2 = 
                    Dict.empty
                        |> Dict.insert 0 100
                        |> Dict.insert 2 200
                        |> Dict.insert 4 300

                Dict.intersection dict1 dict2 
                    |> Dict.values 
                "#
            ),
            &[4, 2],
            &[i64]
        );
    }

    #[test]
    fn difference() {
        assert_evals_to!(
            indoc!(
                r#"
                dict1 : Dict I64 {}
                dict1 = 
                    Dict.empty
                        |> Dict.insert 1 {}
                        |> Dict.insert 2 {}
                        |> Dict.insert 3 {}
                        |> Dict.insert 4 {}
                        |> Dict.insert 5 {}

                dict2 : Dict I64 {}
                dict2 = 
                    Dict.empty
                        |> Dict.insert 0 {}
                        |> Dict.insert 2 {}
                        |> Dict.insert 4 {}

                Dict.difference dict1 dict2 
                    |> Dict.len 
                "#
            ),
            3,
            i64
        );
    }

    #[test]
    fn difference_prefer_first() {
        assert_evals_to!(
            indoc!(
                r#"
                dict1 : Dict I64 I64
                dict1 = 
                    Dict.empty
                        |> Dict.insert 1 1
                        |> Dict.insert 2 2
                        |> Dict.insert 3 3
                        |> Dict.insert 4 4
                        |> Dict.insert 5 5

                dict2 : Dict I64 I64
                dict2 = 
                    Dict.empty
                        |> Dict.insert 0 100
                        |> Dict.insert 2 200
                        |> Dict.insert 4 300

                Dict.difference dict1 dict2 
                    |> Dict.values 
                "#
            ),
            &[5, 3, 1],
            &[i64]
        );
    }

    #[test]
    fn walk_sum_keys() {
        assert_evals_to!(
            indoc!(
                r#"
                dict1 : Dict I64 I64
                dict1 = 
                    Dict.empty
                        |> Dict.insert 1 1
                        |> Dict.insert 2 2
                        |> Dict.insert 3 3
                        |> Dict.insert 4 4
                        |> Dict.insert 5 5

                Dict.walk dict1 (\k, _, a -> k + a) 0 
                "#
            ),
            15,
            i64
        );
    }
}