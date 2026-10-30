//! Optional integrations. Each test module runs only with its feature enabled.

#[cfg(feature = "serde")]
mod serde {
    use mirifici::{Log, SignedLog};
    use serde_test::{Configure, Token, assert_de_tokens_error, assert_tokens};

    type L = Log<f64>;

    #[test]
    fn compact_is_the_stored_float() {
        assert_tokens(&L::from_ln(-2.5).compact(), &[Token::F64(-2.5)]);
        assert_tokens(&L::ZERO.compact(), &[Token::F64(f64::NEG_INFINITY)]);
        assert_tokens(&Log::<f32>::ONE.compact(), &[Token::F32(0.0)]);
    }

    #[test]
    fn readable_writes_infinities_as_strings() {
        assert_tokens(&L::from_ln(-2.5).readable(), &[Token::F64(-2.5)]);
        assert_tokens(&L::ZERO.readable(), &[Token::Str("-inf")]);
        assert_tokens(&L::INFINITY.readable(), &[Token::Str("inf")]);
    }

    #[test]
    fn rejects_nan_and_junk() {
        assert_de_tokens_error::<serde_test::Compact<L>>(
            &[Token::F64(f64::NAN)],
            "value is NaN; Log never stores NaN",
        );
        assert!(serde_json::from_str::<L>("\"zero\"").is_err());
        assert!(serde_json::from_str::<L>("null").is_err());
    }

    #[test]
    fn json_round_trip() {
        for x in [
            L::ZERO,
            L::ONE,
            L::INFINITY,
            L::from_log10(-5000.0),
            L::from_value(0.3),
        ] {
            let s = serde_json::to_string(&x).unwrap();
            assert_eq!(serde_json::from_str::<L>(&s).unwrap(), x, "{s}");
        }
        assert_eq!(serde_json::to_string(&L::ZERO).unwrap(), "\"-inf\"");
        assert_eq!(serde_json::from_str::<L>("-3").unwrap(), L::from_ln(-3.0));
    }

    #[test]
    fn signed_round_trip() {
        let x = SignedLog::new(true, L::from_log10(-5000.0));
        let s = serde_json::to_string(&x).unwrap();
        assert_eq!(serde_json::from_str::<SignedLog>(&s).unwrap(), x);
        assert_eq!(
            serde_json::to_string(&SignedLog::<f64>::ZERO).unwrap(),
            "[false,\"-inf\"]"
        );
    }

    #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
    struct Model {
        #[serde(with = "mirifici::serde_as_value")]
        p: Log,
    }

    #[test]
    fn as_value() {
        let cases = [
            (L::from_value(0.25), r#"{"p":0.25}"#),
            (L::ZERO, r#"{"p":0.0}"#),
            (L::INFINITY, r#"{"p":"inf"}"#),
            (L::from_log10(-5000.0), r#"{"p":"1e-5000"}"#),
            (L::from_value(5e-324), r#"{"p":"4.9406564584127e-324"}"#),
        ];
        for (p, json) in cases {
            let m = Model { p };
            assert_eq!(serde_json::to_string(&m).unwrap(), json);
            assert_eq!(serde_json::from_str::<Model>(json).unwrap(), m);
        }
        assert_eq!(
            serde_json::from_str::<Model>(r#"{"p":1}"#).unwrap(),
            Model { p: L::ONE }
        );
        assert!(serde_json::from_str::<Model>(r#"{"p":-1}"#).is_err());
        assert!(serde_json::from_str::<Model>(r#"{"p":"-1e-5000"}"#).is_err());
    }
}

#[cfg(feature = "num-traits")]
mod num_traits {
    use mirifici::{Log, SignedLog};
    use num_traits::{Bounded, One, Pow, Zero};

    type L = Log<f64>;

    #[test]
    fn identities_and_bounds() {
        assert_eq!(L::zero(), L::ZERO);
        assert!(L::ZERO.is_zero());
        assert_eq!(L::one(), L::ONE);
        assert!(L::ONE.is_one());
        assert_eq!(L::min_value(), L::ZERO);
        assert_eq!(L::max_value(), L::INFINITY);
        assert_eq!(SignedLog::<f64>::zero(), SignedLog::ZERO);
        assert_eq!(SignedLog::<f64>::one(), SignedLog::ONE);
    }

    #[test]
    fn pow_matches_methods() {
        let x = L::from_ln(-2.0);
        assert_eq!(Pow::pow(x, 1.5), x.powf(1.5));
        assert_eq!(Pow::pow(x, 3i32), x.powi(3));
    }

    /// Generic code written against num-traits works with `Log`.
    fn product_of<T: One + core::ops::Mul<Output = T> + Copy>(xs: &[T]) -> T {
        xs.iter().fold(T::one(), |a, &b| a * b)
    }

    #[test]
    fn generic_code() {
        let xs = [L::from_value(2.0), L::from_value(3.0)];
        assert!((product_of(&xs).to_value() - 6.0).abs() < 1e-15);
    }
}

#[cfg(feature = "approx")]
mod approx {
    use approx::{assert_abs_diff_eq, assert_relative_eq, assert_ulps_eq};
    use mirifici::Log;

    type L = Log<f64>;

    #[test]
    fn compares_stored_logs() {
        let a = L::from_value(0.1) + L::from_value(0.2);
        let b = L::from_value(0.3);
        assert_abs_diff_eq!(a, b, epsilon = 1e-15);
        assert_relative_eq!(a, b, max_relative = 1e-15);
        assert_ulps_eq!(a, b, max_ulps = 4);
        assert!(approx::abs_diff_ne!(
            L::from_value(1.0),
            L::from_value(1.001),
            epsilon = 1e-6
        ));
    }

    #[test]
    fn infinities_equal_only_themselves() {
        assert_abs_diff_eq!(L::ZERO, L::ZERO);
        assert_relative_eq!(L::INFINITY, L::INFINITY);
        assert_ulps_eq!(L::ZERO, L::ZERO);
        assert!(approx::abs_diff_ne!(L::ZERO, L::from_ln(-1e300)));
    }
}

#[cfg(feature = "bytemuck")]
mod bytemuck {
    use mirifici::Log;

    #[test]
    fn zeroed_is_one() {
        let z: Log<f64> = bytemuck::Zeroable::zeroed();
        assert_eq!(z, Log::ONE);
    }

    #[test]
    fn cast_slices() {
        let xs = [Log::from_value(0.5f32), Log::ZERO];
        let raw: &[f32] = bytemuck::cast_slice(&xs);
        assert_eq!(raw, &[0.5f32.ln(), f32::NEG_INFINITY]);
        let back: &[Log<f32>] = bytemuck::cast_slice(raw);
        assert_eq!(back, &xs);
    }
}
