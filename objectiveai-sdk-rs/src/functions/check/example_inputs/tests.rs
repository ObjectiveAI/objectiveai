#[cfg(test)]
mod tests {
    use crate::functions::expression::{
        AnyOfInputSchema, ArrayInputSchema, AudioInputSchema, BooleanInputSchema,
        FileInputSchema, ImageInputSchema, InputSchema, IntegerInputSchema, NumberInputSchema,
        ObjectInputSchema, StringInputSchema, VideoInputSchema,
    };
    use crate::functions::check::example_inputs;
    use crate::test_util::index_map;

    fn test(schema: &InputSchema, expected_perms: usize) {
        let perms = example_inputs::permutations(schema);
        assert_eq!(
            perms, expected_perms,
            "permutations mismatch: got {perms}, expected {expected_perms}",
        );

        let expected_len = perms;
        let mut iter = example_inputs::generate(schema);
        for i in 0..expected_len {
            assert!(
                iter.next().is_some(),
                "generator stopped early at item {i}/{expected_len}",
            );
        }
        assert!(
            iter.next().is_none(),
            "generator should have stopped after {expected_len} items",
        );
    }

    // 0 permutations
    #[test]
    fn test_1() {
        test(
            &InputSchema::String(StringInputSchema {
                r#type: Default::default(),
                description: None,
                r#enum: Some(vec![]),
            }),
            0,
        );
    }

    // 2 permutations: wav + mp3
    #[test]
    fn test_2() {
        test(
            &InputSchema::Audio(AudioInputSchema {
                r#type: Default::default(),
                description: None,
            }),
            2,
        );
    }

    // 2 permutations
    #[test]
    fn test_3() {
        test(
            &InputSchema::Boolean(BooleanInputSchema {
                r#type: Default::default(),
                description: None,
            }),
            2,
        );
    }

    // 2 permutations
    #[test]
    fn test_4() {
        test(
            &InputSchema::String(StringInputSchema {
                r#type: Default::default(),
                description: None,
                r#enum: None,
            }),
            2,
        );
    }

    // 2 permutations
    #[test]
    fn test_5() {
        test(
            &InputSchema::Video(VideoInputSchema {
                r#type: Default::default(),
                description: None,
            }),
            2,
        );
    }

    // 3 permutations: zero + min(-5) + max(5)
    #[test]
    fn test_6() {
        test(
            &InputSchema::Integer(IntegerInputSchema {
                r#type: Default::default(),
                description: None,
                minimum: Some(-5),
                maximum: Some(5),
            }),
            3,
        );
    }

    // 3 permutations: zero + random_neg + random_pos
    #[test]
    fn test_7() {
        test(
            &InputSchema::Integer(IntegerInputSchema {
                r#type: Default::default(),
                description: None,
                minimum: None,
                maximum: None,
            }),
            3,
        );
    }

    // 4 permutations: detail None/Auto/Low/High
    #[test]
    fn test_8() {
        test(
            &InputSchema::Image(ImageInputSchema {
                r#type: Default::default(),
                description: None,
            }),
            4,
        );
    }

    // 4 permutations: anyOf [bool(2), audio(2)] → 2 * max(2,2) = 4
    #[test]
    fn test_9() {
        test(
            &InputSchema::AnyOf(AnyOfInputSchema {
                any_of: vec![
                    InputSchema::Boolean(BooleanInputSchema {
                        r#type: Default::default(),
                        description: None,
                    }),
                    InputSchema::Audio(AudioInputSchema {
                        r#type: Default::default(),
                        description: None,
                    }),
                ],
            }),
            4,
        );
    }

    // 5 permutations: zero + random_neg + random_pos + decimal_neg + decimal_pos
    #[test]
    fn test_10() {
        test(
            &InputSchema::Number(NumberInputSchema {
                r#type: Default::default(),
                description: None,
                minimum: None,
                maximum: None,
            }),
            5,
        );
    }

    // 4 permutations: array of bool(2), min=1 max=2 → mid=1, lengths={1,2}, mult=2 → 2*2 = 4
    #[test]
    fn test_11() {
        test(
            &InputSchema::Array(ArrayInputSchema {
                r#type: Default::default(),
                description: None,
                min_items: Some(1),
                max_items: Some(2),
                items: Box::new(InputSchema::Boolean(BooleanInputSchema {
                    r#type: Default::default(),
                    description: None,
                })),
            }),
            4,
        );
    }

    // 9 permutations: anyOf [bool(2), string(2), integer(3)] → 3 * max(2,2,3) = 9
    #[test]
    fn test_12() {
        test(
            &InputSchema::AnyOf(AnyOfInputSchema {
                any_of: vec![
                    InputSchema::Boolean(BooleanInputSchema {
                        r#type: Default::default(),
                        description: None,
                    }),
                    InputSchema::String(StringInputSchema {
                        r#type: Default::default(),
                        description: None,
                        r#enum: None,
                    }),
                    InputSchema::Integer(IntegerInputSchema {
                        r#type: Default::default(),
                        description: None,
                        minimum: None,
                        maximum: None,
                    }),
                ],
            }),
            9,
        );
    }

    // 3 permutations: {str(2), bool(2), int(3)} all required → max(2,2,3) = 3
    #[test]
    fn test_13() {
        test(
            &InputSchema::Object(ObjectInputSchema {
                r#type: Default::default(),
                description: None,
                properties: index_map! {
                    "a" => InputSchema::String(StringInputSchema {
                        r#type: Default::default(),
                        description: None,
                        r#enum: None,
                    }),
                    "b" => InputSchema::Boolean(BooleanInputSchema {
                        r#type: Default::default(),
                        description: None,
                    }),
                    "c" => InputSchema::Integer(IntegerInputSchema {
                        r#type: Default::default(),
                        description: None,
                        minimum: None,
                        maximum: None,
                    }),
                },
                required: Some(vec![
                    "a".to_string(),
                    "b".to_string(),
                    "c".to_string(),
                ]),
            }),
            3,
        );
    }

    // 6 permutations: array of bool(2), min=0 max=3 → mid=1, lengths={0,1,3}, mult=3 → 2*3 = 6
    #[test]
    fn test_14() {
        test(
            &InputSchema::Array(ArrayInputSchema {
                r#type: Default::default(),
                description: None,
                min_items: Some(0),
                max_items: Some(3),
                items: Box::new(InputSchema::Boolean(BooleanInputSchema {
                    r#type: Default::default(),
                    description: None,
                })),
            }),
            6,
        );
    }

    // 16 permutations: file (2^4 optional fields)
    #[test]
    fn test_15() {
        test(
            &InputSchema::File(FileInputSchema {
                r#type: Default::default(),
                description: None,
            }),
            16,
        );
    }

    // 4 permutations: {name(2), active(2), nickname(optional 2*2=4)} → max(2,2,4) = 4
    #[test]
    fn test_16() {
        test(
            &InputSchema::Object(ObjectInputSchema {
                r#type: Default::default(),
                description: None,
                properties: index_map! {
                    "name" => InputSchema::String(StringInputSchema {
                        r#type: Default::default(),
                        description: None,
                        r#enum: None,
                    }),
                    "active" => InputSchema::Boolean(BooleanInputSchema {
                        r#type: Default::default(),
                        description: None,
                    }),
                    "nickname" => InputSchema::String(StringInputSchema {
                        r#type: Default::default(),
                        description: None,
                        r#enum: None,
                    }),
                },
                required: Some(vec!["name".to_string(), "active".to_string()]),
            }),
            4,
        );
    }

    // 6 permutations: array of string(enum 3), min=2 max=3 → mid=2, lengths={2,3}, mult=2 → 3*2 = 6
    #[test]
    fn test_17() {
        test(
            &InputSchema::Array(ArrayInputSchema {
                r#type: Default::default(),
                description: None,
                min_items: Some(2),
                max_items: Some(3),
                items: Box::new(InputSchema::String(StringInputSchema {
                    r#type: Default::default(),
                    description: None,
                    r#enum: Some(vec![
                        "x".to_string(),
                        "y".to_string(),
                        "z".to_string(),
                    ]),
                })),
            }),
            6,
        );
    }

    // 4 permutations: {a, b, c} all optional booleans → max(4,4,4) = 4
    #[test]
    fn test_18() {
        test(
            &InputSchema::Object(ObjectInputSchema {
                r#type: Default::default(),
                description: None,
                properties: index_map! {
                    "a" => InputSchema::Boolean(BooleanInputSchema {
                        r#type: Default::default(),
                        description: None,
                    }),
                    "b" => InputSchema::Boolean(BooleanInputSchema {
                        r#type: Default::default(),
                        description: None,
                    }),
                    "c" => InputSchema::Boolean(BooleanInputSchema {
                        r#type: Default::default(),
                        description: None,
                    }),
                },
                required: None,
            }),
            4,
        );
    }

    // 32 permutations: {file(optional 16*2=32), image(optional 4*2=8)} → max(32,8) = 32
    #[test]
    fn test_19() {
        test(
            &InputSchema::Object(ObjectInputSchema {
                r#type: Default::default(),
                description: None,
                properties: index_map! {
                    "a" => InputSchema::File(FileInputSchema {
                        r#type: Default::default(),
                        description: None,
                    }),
                    "b" => InputSchema::Image(ImageInputSchema {
                        r#type: Default::default(),
                        description: None,
                    }),
                },
                required: None,
            }),
            32,
        );
    }

    // 4 permutations: 5 optional booleans → max(4,4,4,4,4) = 4
    #[test]
    fn test_20() {
        test(
            &InputSchema::Object(ObjectInputSchema {
                r#type: Default::default(),
                description: None,
                properties: index_map! {
                    "a" => InputSchema::Boolean(BooleanInputSchema {
                        r#type: Default::default(),
                        description: None,
                    }),
                    "b" => InputSchema::Boolean(BooleanInputSchema {
                        r#type: Default::default(),
                        description: None,
                    }),
                    "c" => InputSchema::Boolean(BooleanInputSchema {
                        r#type: Default::default(),
                        description: None,
                    }),
                    "d" => InputSchema::Boolean(BooleanInputSchema {
                        r#type: Default::default(),
                        description: None,
                    }),
                    "e" => InputSchema::Boolean(BooleanInputSchema {
                        r#type: Default::default(),
                        description: None,
                    }),
                },
                required: None,
            }),
            4,
        );
    }

    // --- Array tests ---

    // 4 permutations: array of audio(2), min=0 max=1 → mid=0, lengths={0,1}, mult=2 → 2*2 = 4
    #[test]
    fn test_21() {
        test(
            &InputSchema::Array(ArrayInputSchema {
                r#type: Default::default(),
                description: None,
                min_items: Some(0),
                max_items: Some(1),
                items: Box::new(InputSchema::Audio(AudioInputSchema {
                    r#type: Default::default(),
                    description: None,
                })),
            }),
            4,
        );
    }

    // 4 permutations: array of image(4), min=1 max=1 → min==max, mult=1 → 4*1 = 4
    #[test]
    fn test_22() {
        test(
            &InputSchema::Array(ArrayInputSchema {
                r#type: Default::default(),
                description: None,
                min_items: Some(1),
                max_items: Some(1),
                items: Box::new(InputSchema::Image(ImageInputSchema {
                    r#type: Default::default(),
                    description: None,
                })),
            }),
            4,
        );
    }

    // 6 permutations: array of integer(3), min=1 max=2 → mid=1, lengths={1,2}, mult=2 → 3*2 = 6
    #[test]
    fn test_23() {
        test(
            &InputSchema::Array(ArrayInputSchema {
                r#type: Default::default(),
                description: None,
                min_items: Some(1),
                max_items: Some(2),
                items: Box::new(InputSchema::Integer(IntegerInputSchema {
                    r#type: Default::default(),
                    description: None,
                    minimum: None,
                    maximum: None,
                })),
            }),
            6,
        );
    }

    // 16 permutations: array of file(16), min=1 max=1 → min==max, mult=1 → 16*1 = 16
    #[test]
    fn test_24() {
        test(
            &InputSchema::Array(ArrayInputSchema {
                r#type: Default::default(),
                description: None,
                min_items: Some(1),
                max_items: Some(1),
                items: Box::new(InputSchema::File(FileInputSchema {
                    r#type: Default::default(),
                    description: None,
                })),
            }),
            16,
        );
    }

    // 15 permutations: array of number(5), min=1 max=3 → mid=2, lengths={1,2,3}, mult=3 → 5*3 = 15
    #[test]
    fn test_25() {
        test(
            &InputSchema::Array(ArrayInputSchema {
                r#type: Default::default(),
                description: None,
                min_items: Some(1),
                max_items: Some(3),
                items: Box::new(InputSchema::Number(NumberInputSchema {
                    r#type: Default::default(),
                    description: None,
                    minimum: None,
                    maximum: None,
                })),
            }),
            15,
        );
    }

    // --- Object-with-array tests ---

    // 2 permutations: {tags: array(bool, 1..1)=2*1=2} required → max(2) = 2
    #[test]
    fn test_26() {
        test(
            &InputSchema::Object(ObjectInputSchema {
                r#type: Default::default(),
                description: None,
                properties: index_map! {
                    "tags" => InputSchema::Array(ArrayInputSchema {
                        r#type: Default::default(),
                        description: None,
                        min_items: Some(1),
                        max_items: Some(1),
                        items: Box::new(InputSchema::Boolean(BooleanInputSchema {
                            r#type: Default::default(),
                            description: None,
                        })),
                    }),
                },
                required: Some(vec!["tags".to_string()]),
            }),
            2,
        );
    }

    // 3 permutations: {scores: array(int, 1..1)=3*1=3, active: bool=2} required → max(3,2) = 3
    #[test]
    fn test_27() {
        test(
            &InputSchema::Object(ObjectInputSchema {
                r#type: Default::default(),
                description: None,
                properties: index_map! {
                    "scores" => InputSchema::Array(ArrayInputSchema {
                        r#type: Default::default(),
                        description: None,
                        min_items: Some(1),
                        max_items: Some(1),
                        items: Box::new(InputSchema::Integer(IntegerInputSchema {
                            r#type: Default::default(),
                            description: None,
                            minimum: None,
                            maximum: None,
                        })),
                    }),
                    "active" => InputSchema::Boolean(BooleanInputSchema {
                        r#type: Default::default(),
                        description: None,
                    }),
                },
                required: Some(vec!["scores".to_string(), "active".to_string()]),
            }),
            3,
        );
    }

    // 4 permutations: {name: str(2), items: array(bool, 1..2)=2*2=4} required → max(2,4) = 4
    #[test]
    fn test_28() {
        test(
            &InputSchema::Object(ObjectInputSchema {
                r#type: Default::default(),
                description: None,
                properties: index_map! {
                    "name" => InputSchema::String(StringInputSchema {
                        r#type: Default::default(),
                        description: None,
                        r#enum: None,
                    }),
                    "items" => InputSchema::Array(ArrayInputSchema {
                        r#type: Default::default(),
                        description: None,
                        min_items: Some(1),
                        max_items: Some(2),
                        items: Box::new(InputSchema::Boolean(BooleanInputSchema {
                            r#type: Default::default(),
                            description: None,
                        })),
                    }),
                },
                required: Some(vec!["name".to_string(), "items".to_string()]),
            }),
            4,
        );
    }

    // 6 permutations: {items: array(enum3, 1..2)=3*2=6, flag: bool(optional)=4} → max(6,4) = 6
    #[test]
    fn test_29() {
        test(
            &InputSchema::Object(ObjectInputSchema {
                r#type: Default::default(),
                description: None,
                properties: index_map! {
                    "items" => InputSchema::Array(ArrayInputSchema {
                        r#type: Default::default(),
                        description: None,
                        min_items: Some(1),
                        max_items: Some(2),
                        items: Box::new(InputSchema::String(StringInputSchema {
                            r#type: Default::default(),
                            description: None,
                            r#enum: Some(vec![
                                "a".to_string(),
                                "b".to_string(),
                                "c".to_string(),
                            ]),
                        })),
                    }),
                    "flag" => InputSchema::Boolean(BooleanInputSchema {
                        r#type: Default::default(),
                        description: None,
                    }),
                },
                required: Some(vec!["items".to_string()]),
            }),
            6,
        );
    }

    // 16 permutations: {images: array(image, 1..1)=4*1=4, files: array(file, 1..1)=16*1=16} required → max(4,16) = 16
    #[test]
    fn test_30() {
        test(
            &InputSchema::Object(ObjectInputSchema {
                r#type: Default::default(),
                description: None,
                properties: index_map! {
                    "images" => InputSchema::Array(ArrayInputSchema {
                        r#type: Default::default(),
                        description: None,
                        min_items: Some(1),
                        max_items: Some(1),
                        items: Box::new(InputSchema::Image(ImageInputSchema {
                            r#type: Default::default(),
                            description: None,
                        })),
                    }),
                    "files" => InputSchema::Array(ArrayInputSchema {
                        r#type: Default::default(),
                        description: None,
                        min_items: Some(1),
                        max_items: Some(1),
                        items: Box::new(InputSchema::File(FileInputSchema {
                            r#type: Default::default(),
                            description: None,
                        })),
                    }),
                },
                required: Some(vec!["images".to_string(), "files".to_string()]),
            }),
            16,
        );
    }

    // --- Array: min set, no max (range = min..=min+10) ---

    // 6 permutations: array of audio(2), min=0 → 0..=10, mid=5, lengths={0,5,10}, mult=3 → 2*3 = 6
    #[test]
    fn test_31() {
        test(
            &InputSchema::Array(ArrayInputSchema {
                r#type: Default::default(),
                description: None,
                min_items: Some(0),
                max_items: None,
                items: Box::new(InputSchema::Audio(AudioInputSchema {
                    r#type: Default::default(),
                    description: None,
                })),
            }),
            6,
        );
    }

    // 6 permutations: array of bool(2), min=0 → 0..=10, mid=5, lengths={0,5,10}, mult=3 → 2*3 = 6
    #[test]
    fn test_32() {
        test(
            &InputSchema::Array(ArrayInputSchema {
                r#type: Default::default(),
                description: None,
                min_items: Some(0),
                max_items: None,
                items: Box::new(InputSchema::Boolean(BooleanInputSchema {
                    r#type: Default::default(),
                    description: None,
                })),
            }),
            6,
        );
    }

    // 6 permutations: array of audio(2), min=3 → 3..=13, mid=8, lengths={3,8,13}, mult=3 → 2*3 = 6
    #[test]
    fn test_33() {
        test(
            &InputSchema::Array(ArrayInputSchema {
                r#type: Default::default(),
                description: None,
                min_items: Some(3),
                max_items: None,
                items: Box::new(InputSchema::Audio(AudioInputSchema {
                    r#type: Default::default(),
                    description: None,
                })),
            }),
            6,
        );
    }

    // 6 permutations: array of video(2), min=1 → 1..=11, mid=6, lengths={1,6,11}, mult=3 → 2*3 = 6
    #[test]
    fn test_34() {
        test(
            &InputSchema::Array(ArrayInputSchema {
                r#type: Default::default(),
                description: None,
                min_items: Some(1),
                max_items: None,
                items: Box::new(InputSchema::Video(VideoInputSchema {
                    r#type: Default::default(),
                    description: None,
                })),
            }),
            6,
        );
    }

    // 6 permutations: array of integer(1..10)(2), min=2 → 2..=12, mid=7, lengths={2,7,12}, mult=3 → 2*3 = 6
    #[test]
    fn test_35() {
        test(
            &InputSchema::Array(ArrayInputSchema {
                r#type: Default::default(),
                description: None,
                min_items: Some(2),
                max_items: None,
                items: Box::new(InputSchema::Integer(IntegerInputSchema {
                    r#type: Default::default(),
                    description: None,
                    minimum: Some(1),
                    maximum: Some(10),
                })),
            }),
            6,
        );
    }

    // --- Array: max set, no min (range = max.saturating_sub(10)..=max) ---

    // 2 permutations: array of bool(2), max=0 → 0..=0, min==max, mult=1 → 2*1 = 2
    #[test]
    fn test_36() {
        test(
            &InputSchema::Array(ArrayInputSchema {
                r#type: Default::default(),
                description: None,
                min_items: None,
                max_items: Some(0),
                items: Box::new(InputSchema::Boolean(BooleanInputSchema {
                    r#type: Default::default(),
                    description: None,
                })),
            }),
            2,
        );
    }

    // 6 permutations: array of audio(2), max=5 → 0..=5, mid=2, lengths={0,2,5}, mult=3 → 2*3 = 6
    #[test]
    fn test_37() {
        test(
            &InputSchema::Array(ArrayInputSchema {
                r#type: Default::default(),
                description: None,
                min_items: None,
                max_items: Some(5),
                items: Box::new(InputSchema::Audio(AudioInputSchema {
                    r#type: Default::default(),
                    description: None,
                })),
            }),
            6,
        );
    }

    // 6 permutations: array of bool(2), max=3 → 0..=3, mid=1, lengths={0,1,3}, mult=3 → 2*3 = 6
    #[test]
    fn test_38() {
        test(
            &InputSchema::Array(ArrayInputSchema {
                r#type: Default::default(),
                description: None,
                min_items: None,
                max_items: Some(3),
                items: Box::new(InputSchema::Boolean(BooleanInputSchema {
                    r#type: Default::default(),
                    description: None,
                })),
            }),
            6,
        );
    }

    // 6 permutations: array of audio(2), max=15 → 5..=15, mid=10, lengths={5,10,15}, mult=3 → 2*3 = 6
    #[test]
    fn test_39() {
        test(
            &InputSchema::Array(ArrayInputSchema {
                r#type: Default::default(),
                description: None,
                min_items: None,
                max_items: Some(15),
                items: Box::new(InputSchema::Audio(AudioInputSchema {
                    r#type: Default::default(),
                    description: None,
                })),
            }),
            6,
        );
    }

    // 6 permutations: array of bool(2), max=8 → 0..=8, mid=4, lengths={0,4,8}, mult=3 → 2*3 = 6
    #[test]
    fn test_40() {
        test(
            &InputSchema::Array(ArrayInputSchema {
                r#type: Default::default(),
                description: None,
                min_items: None,
                max_items: Some(8),
                items: Box::new(InputSchema::Boolean(BooleanInputSchema {
                    r#type: Default::default(),
                    description: None,
                })),
            }),
            6,
        );
    }

    // --- Array: no min, no max (range = 0..=10) ---

    // 6 permutations: array of audio(2) → 0..=10, mid=5, lengths={0,5,10}, mult=3 → 2*3 = 6
    #[test]
    fn test_41() {
        test(
            &InputSchema::Array(ArrayInputSchema {
                r#type: Default::default(),
                description: None,
                min_items: None,
                max_items: None,
                items: Box::new(InputSchema::Audio(AudioInputSchema {
                    r#type: Default::default(),
                    description: None,
                })),
            }),
            6,
        );
    }

    // 6 permutations: array of bool(2) → 0..=10, mid=5, lengths={0,5,10}, mult=3 → 2*3 = 6
    #[test]
    fn test_42() {
        test(
            &InputSchema::Array(ArrayInputSchema {
                r#type: Default::default(),
                description: None,
                min_items: None,
                max_items: None,
                items: Box::new(InputSchema::Boolean(BooleanInputSchema {
                    r#type: Default::default(),
                    description: None,
                })),
            }),
            6,
        );
    }

    // 12 permutations: array of image(4) → 0..=10, mid=5, lengths={0,5,10}, mult=3 → 4*3 = 12
    #[test]
    fn test_43() {
        test(
            &InputSchema::Array(ArrayInputSchema {
                r#type: Default::default(),
                description: None,
                min_items: None,
                max_items: None,
                items: Box::new(InputSchema::Image(ImageInputSchema {
                    r#type: Default::default(),
                    description: None,
                })),
            }),
            12,
        );
    }

    // 9 permutations: array of integer(no bounds)(3) → 0..=10, mid=5, lengths={0,5,10}, mult=3 → 3*3 = 9
    #[test]
    fn test_44() {
        test(
            &InputSchema::Array(ArrayInputSchema {
                r#type: Default::default(),
                description: None,
                min_items: None,
                max_items: None,
                items: Box::new(InputSchema::Integer(IntegerInputSchema {
                    r#type: Default::default(),
                    description: None,
                    minimum: None,
                    maximum: None,
                })),
            }),
            9,
        );
    }

    // 6 permutations: array of video(2) → 0..=10, mid=5, lengths={0,5,10}, mult=3 → 2*3 = 6
    #[test]
    fn test_45() {
        test(
            &InputSchema::Array(ArrayInputSchema {
                r#type: Default::default(),
                description: None,
                min_items: None,
                max_items: None,
                items: Box::new(InputSchema::Video(VideoInputSchema {
                    r#type: Default::default(),
                    description: None,
                })),
            }),
            6,
        );
    }

    // --- Array: multiplier edge cases ---

    // 2 permutations: array of bool(2), min=0 max=0 → gap=0, mult=1 → 2*1 = 2
    // (old system: 1+1 prefix = 2)
    #[test]
    fn test_46() {
        test(
            &InputSchema::Array(ArrayInputSchema {
                r#type: Default::default(),
                description: None,
                min_items: Some(0),
                max_items: Some(0),
                items: Box::new(InputSchema::Boolean(BooleanInputSchema {
                    r#type: Default::default(),
                    description: None,
                })),
            }),
            2,
        );
    }

    // 2 permutations: array of bool(2), min=5 max=5 → gap=0, mult=1 → 2*1 = 2
    // (old system: 2^5=32, +1 prefix = 33)
    #[test]
    fn test_47() {
        test(
            &InputSchema::Array(ArrayInputSchema {
                r#type: Default::default(),
                description: None,
                min_items: Some(5),
                max_items: Some(5),
                items: Box::new(InputSchema::Boolean(BooleanInputSchema {
                    r#type: Default::default(),
                    description: None,
                })),
            }),
            2,
        );
    }

    // 4 permutations: array of bool(2), min=3 max=4 → gap=1, mult=2 → 2*2 = 4
    // (old system: 2^3+2^4=24, +2 prefix = 26)
    #[test]
    fn test_48() {
        test(
            &InputSchema::Array(ArrayInputSchema {
                r#type: Default::default(),
                description: None,
                min_items: Some(3),
                max_items: Some(4),
                items: Box::new(InputSchema::Boolean(BooleanInputSchema {
                    r#type: Default::default(),
                    description: None,
                })),
            }),
            4,
        );
    }

    // 6 permutations: array of bool(2), min=2 max=4 → gap=2, mult=3 → 2*3 = 6
    // (old system: 2^2+2^3+2^4=28, +2 prefix = 30)
    #[test]
    fn test_49() {
        test(
            &InputSchema::Array(ArrayInputSchema {
                r#type: Default::default(),
                description: None,
                min_items: Some(2),
                max_items: Some(4),
                items: Box::new(InputSchema::Boolean(BooleanInputSchema {
                    r#type: Default::default(),
                    description: None,
                })),
            }),
            6,
        );
    }

    // 9 permutations: array of int(3), min=0 max=4 → gap=4, mult=3 → 3*3 = 9
    // (old system: 1+3+9+27+81=121, +2 prefix = 123)
    #[test]
    fn test_50() {
        test(
            &InputSchema::Array(ArrayInputSchema {
                r#type: Default::default(),
                description: None,
                min_items: Some(0),
                max_items: Some(4),
                items: Box::new(InputSchema::Integer(IntegerInputSchema {
                    r#type: Default::default(),
                    description: None,
                    minimum: None,
                    maximum: None,
                })),
            }),
            9,
        );
    }

    // 6 permutations: array of bool(2), min=10 max=11 → gap=1, mult=2 → 2*2 = 4
    // (old system: 2^10+2^11=3072, +2 prefix = 3074)
    #[test]
    fn test_51() {
        test(
            &InputSchema::Array(ArrayInputSchema {
                r#type: Default::default(),
                description: None,
                min_items: Some(10),
                max_items: Some(11),
                items: Box::new(InputSchema::Boolean(BooleanInputSchema {
                    r#type: Default::default(),
                    description: None,
                })),
            }),
            4,
        );
    }

    // 6 permutations: array of bool(2), min=0 max=100 → gap=100, mult=3 → 2*3 = 6
    // (old system: astronomically large)
    #[test]
    fn test_52() {
        test(
            &InputSchema::Array(ArrayInputSchema {
                r#type: Default::default(),
                description: None,
                min_items: Some(0),
                max_items: Some(100),
                items: Box::new(InputSchema::Boolean(BooleanInputSchema {
                    r#type: Default::default(),
                    description: None,
                })),
            }),
            6,
        );
    }

    // --- High permutation tests (>100) ---

    // 108 permutations: array(array(array(image(4), 1..3), 1..3), 1..3)
    // high depth: 3 nested arrays. 4*3=12 → 12*3=36 → 36*3=108
    #[test]
    fn test_53() {
        test(
            &InputSchema::Array(ArrayInputSchema {
                r#type: Default::default(),
                description: None,
                min_items: Some(1),
                max_items: Some(3),
                items: Box::new(InputSchema::Array(ArrayInputSchema {
                    r#type: Default::default(),
                    description: None,
                    min_items: Some(1),
                    max_items: Some(3),
                    items: Box::new(InputSchema::Array(ArrayInputSchema {
                        r#type: Default::default(),
                        description: None,
                        min_items: Some(1),
                        max_items: Some(3),
                        items: Box::new(InputSchema::Image(ImageInputSchema {
                            r#type: Default::default(),
                            description: None,
                        })),
                    })),
                })),
            }),
            108,
        );
    }

    // 384 permutations: array(anyOf[8 types], 1..5)
    // high width: 8 anyOf variants. 8*max(2,2,3,5,4,16,2,2)=128, *3=384
    #[test]
    fn test_54() {
        test(
            &InputSchema::Array(ArrayInputSchema {
                r#type: Default::default(),
                description: None,
                min_items: Some(1),
                max_items: Some(5),
                items: Box::new(InputSchema::AnyOf(AnyOfInputSchema {
                    any_of: vec![
                        InputSchema::Boolean(BooleanInputSchema {
                            r#type: Default::default(),
                            description: None,
                        }),
                        InputSchema::String(StringInputSchema {
                            r#type: Default::default(),
                            description: None,
                            r#enum: None,
                        }),
                        InputSchema::Integer(IntegerInputSchema {
                            r#type: Default::default(),
                            description: None,
                            minimum: None,
                            maximum: None,
                        }),
                        InputSchema::Number(NumberInputSchema {
                            r#type: Default::default(),
                            description: None,
                            minimum: None,
                            maximum: None,
                        }),
                        InputSchema::Image(ImageInputSchema {
                            r#type: Default::default(),
                            description: None,
                        }),
                        InputSchema::File(FileInputSchema {
                            r#type: Default::default(),
                            description: None,
                        }),
                        InputSchema::Audio(AudioInputSchema {
                            r#type: Default::default(),
                            description: None,
                        }),
                        InputSchema::Video(VideoInputSchema {
                            r#type: Default::default(),
                            description: None,
                        }),
                    ],
                })),
            }),
            384,
        );
    }

    // 128 permutations: {a_opt: {b_opt: {c_opt: {d: file(16)}}}}
    // high depth: 4 nested objects. 16 → opt 32 → opt 64 → opt 128
    #[test]
    fn test_55() {
        test(
            &InputSchema::Object(ObjectInputSchema {
                r#type: Default::default(),
                description: None,
                properties: index_map! {
                    "a" => InputSchema::Object(ObjectInputSchema {
                        r#type: Default::default(),
                        description: None,
                        properties: index_map! {
                            "b" => InputSchema::Object(ObjectInputSchema {
                                r#type: Default::default(),
                                description: None,
                                properties: index_map! {
                                    "c" => InputSchema::Object(ObjectInputSchema {
                                        r#type: Default::default(),
                                        description: None,
                                        properties: index_map! {
                                            "d" => InputSchema::File(FileInputSchema {
                                                r#type: Default::default(),
                                                description: None,
                                            }),
                                        },
                                        required: Some(vec!["d".to_string()]),
                                    }),
                                },
                                required: None,
                            }),
                        },
                        required: None,
                    }),
                },
                required: None,
            }),
            128,
        );
    }

    // 192 permutations: 8 fields, h optional array(anyOf[file,image], 2..8)
    // high width: 8 object fields. h_opt: 2*16=32, *3=96, opt 192
    #[test]
    fn test_56() {
        test(
            &InputSchema::Object(ObjectInputSchema {
                r#type: Default::default(),
                description: None,
                properties: index_map! {
                    "a" => InputSchema::File(FileInputSchema {
                        r#type: Default::default(),
                        description: None,
                    }),
                    "b" => InputSchema::Image(ImageInputSchema {
                        r#type: Default::default(),
                        description: None,
                    }),
                    "c" => InputSchema::Number(NumberInputSchema {
                        r#type: Default::default(),
                        description: None,
                        minimum: None,
                        maximum: None,
                    }),
                    "d" => InputSchema::Integer(IntegerInputSchema {
                        r#type: Default::default(),
                        description: None,
                        minimum: None,
                        maximum: None,
                    }),
                    "e" => InputSchema::Boolean(BooleanInputSchema {
                        r#type: Default::default(),
                        description: None,
                    }),
                    "f" => InputSchema::String(StringInputSchema {
                        r#type: Default::default(),
                        description: None,
                        r#enum: None,
                    }),
                    "g" => InputSchema::Audio(AudioInputSchema {
                        r#type: Default::default(),
                        description: None,
                    }),
                    "h" => InputSchema::Array(ArrayInputSchema {
                        r#type: Default::default(),
                        description: None,
                        min_items: Some(2),
                        max_items: Some(8),
                        items: Box::new(InputSchema::AnyOf(AnyOfInputSchema {
                            any_of: vec![
                                InputSchema::File(FileInputSchema {
                                    r#type: Default::default(),
                                    description: None,
                                }),
                                InputSchema::Image(ImageInputSchema {
                                    r#type: Default::default(),
                                    description: None,
                                }),
                            ],
                        })),
                    }),
                },
                required: Some(vec![
                    "a".to_string(),
                    "b".to_string(),
                    "c".to_string(),
                    "d".to_string(),
                    "e".to_string(),
                    "f".to_string(),
                    "g".to_string(),
                ]),
            }),
            192,
        );
    }

    // 144 permutations: {items: array(anyOf[file,image,number], 1..5), name: string, active: bool}
    // middleground: moderate depth (obj→array→anyOf→leaves) + moderate width (3 fields, 3 variants)
    // anyOf: 3*16=48, array: 48*3=144, max(144,2,2)=144
    #[test]
    fn test_57() {
        test(
            &InputSchema::Object(ObjectInputSchema {
                r#type: Default::default(),
                description: None,
                properties: index_map! {
                    "items" => InputSchema::Array(ArrayInputSchema {
                        r#type: Default::default(),
                        description: None,
                        min_items: Some(1),
                        max_items: Some(5),
                        items: Box::new(InputSchema::AnyOf(AnyOfInputSchema {
                            any_of: vec![
                                InputSchema::File(FileInputSchema {
                                    r#type: Default::default(),
                                    description: None,
                                }),
                                InputSchema::Image(ImageInputSchema {
                                    r#type: Default::default(),
                                    description: None,
                                }),
                                InputSchema::Number(NumberInputSchema {
                                    r#type: Default::default(),
                                    description: None,
                                    minimum: None,
                                    maximum: None,
                                }),
                            ],
                        })),
                    }),
                    "name" => InputSchema::String(StringInputSchema {
                        r#type: Default::default(),
                        description: None,
                        r#enum: None,
                    }),
                    "active" => InputSchema::Boolean(BooleanInputSchema {
                        r#type: Default::default(),
                        description: None,
                    }),
                },
                required: Some(vec![
                    "items".to_string(),
                    "name".to_string(),
                    "active".to_string(),
                ]),
            }),
            144,
        );
    }
}
