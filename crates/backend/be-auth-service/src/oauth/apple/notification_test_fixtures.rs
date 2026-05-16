//! Cryptographic fixtures and helpers for Apple notification tests.
//!
//! Lives alongside `mod.rs` (gated behind `#[cfg(test)]`) rather than
//! inline in the production source so the verifier file stays focussed
//! on production code. None of these symbols are compiled outside of
//! the test build.
//!
//! The two RSA keypairs were generated once for this test suite — the
//! `NOTIF_*` pair represents the key Apple "publishes" via JWKS, and
//! the `FORGED_*` pair backs negative-path tests that exercise the
//! signature-verification failure mode without any chance of
//! accidentally validating against the legitimate key.

use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header};

/// RS256 test keypair specifically for notification tests.
/// Generated once, baked in. The matching public modulus / exponent
/// (used to build the test JWK) come below as base64url strings.
pub(super) const NOTIF_PRIVATE_PEM: &str = "-----BEGIN PRIVATE KEY-----\n\
MIIEvQIBADANBgkqhkiG9w0BAQEFAASCBKcwggSjAgEAAoIBAQCnPmkSC3Undp5a\n\
Hkv5kVa6Px5joa6htjUxguImLjRGk2ihLFZ9bEpP2jlrnO+kIqlYqJHt45w23QRh\n\
d8CGnLDx94E9AQo3ezmDQmsCzred+h6T5IsC8YpLIJkkuRNqEsFM6LCYkT3PzXYn\n\
adz36pBXa2jFcm8Q3MzaE/YRJ/zvxj4WNvD0E62iT/k+9oeL/Tz1HIN+kMg+CvhB\n\
hYVJvaFHgCQGqVgYM7ZcO8xBENkARa0jZ6aGHIJ5sMXxZOBfZUQDDFwsVXEkucRM\n\
xJ1y8R4Wx9baj3IU7eYrfKabdmCve1DAF6phdomTqhFOkNTUQe2OT2UkHKgfNld+\n\
Kmm/aEglAgMBAAECggEACxxoWpyMQfztdEtC/Oec/W6oFDJvqaqzSgDKCYNUUcQF\n\
VX/elyKUpU14NyAMA7mRyqQYXdebT0HLwrX0An1yfADXgzgId7smDQRim6MkK8is\n\
M2MhoSZu66LUyc3J7mgkk3l6EgjENIQP73pnBcl9oXEvWr7JhjNFIM/mryka8sr7\n\
NkWY9JGLGgQzopMYsZwGxGnAc164ji2K/b49yhCEzXoe//vOMI0fi8su+DOvOJgP\n\
zBZAss/BxidPylfk7KCgFO+mWURS5N81YqMnx7N86ROztw97xU1aWYbLJDsXwtzO\n\
YksikwG00h0sDy7m76P74DrR6QlHPXJqw2rLtRIPgQKBgQDneZ+MWmH9CMDlhmxk\n\
jxCXHfA8eWTY4eJusUTxKiTLUcjYB/Yj/xKjtxjGspDi90HQbrjKxgMtJ8EUb+Dr\n\
/6VJIX8MjctYIeBAYDUZ5a4MKJy0m6JFSO0mIexBbCK6JTuzKj/UqOtbYZmWG+Wr\n\
fORy6Av6KY9s8onX1zhDFkx3sQKBgQC49p66qy+mv3b9r3Eh411Hn2dCIgfWnNQn\n\
iGssvb3Tm7P56KmrFNGTALLpIqDrycrOd+JvgJJrbszjnu6njhupLgDUU/Iodo4f\n\
gjmHYcutWufxDA5Jj5yU1DopCNwtL8k+Y05vaBfzDo08vDImuOnLxq2YnP7PTI/L\n\
KBbGH+QotQKBgHHabbKQRkA6TP7YVnpDsCpULHTiVMskl8ZQZROl4gErkflIOTZN\n\
YPKrvYEGFaO9cF7ABx6dtLRCKIMP4HbUAI1u71nSaKFJ0E55w8SgJzKNyz4+its/\n\
Wn32E4m+UXpzk+C1OD42c8U3xV6DDD2EKa2nGzUJuiUhStGiZ2cAEc6RAoGBAKrx\n\
872eXUoFhtnrfenOEvYREwQCI7Br/YAUCsmtC3Y5X1tHdxhRA2iTqsbhZEzHkZLF\n\
JhfbgnecTezJhNSC+HmhtM6ITzSqbawdVUIUVoP/koIrnEDMY/EBPEeUkrmIgrwQ\n\
V/uK/yd6eXp6jPdQy3O0SdjUsIOyxOsEQBgYfWxJAoGAFJKi7n2DwdvPUQNq6RcD\n\
JwNX+vBmQirDEciFbAKB817MVDhBEU2/s19+qLXEyBK6qWqA74U0N0o7yAc4H42L\n\
8+ioGU+1BH6eOfrhjrAk6AlomS5R8YDPnzGc/t7kgOoJRCloANMwk26QtekjFiWn\n\
CZ0z06sb8BZvzf9fw1FmMJ4=\n\
-----END PRIVATE KEY-----\n";

/// Modulus (`n`) of [`NOTIF_PRIVATE_PEM`]'s public component,
/// base64url-encoded without padding. Together with `e=AQAB` this
/// forms the JWK Apple would publish on `/auth/keys`.
pub(super) const NOTIF_PUBLIC_N: &str = "pz5pEgt1J3aeWh5L-ZFWuj8eY6GuobY1MYLiJi40RpNooSxWfWxKT9o5a5zvpCKpWKiR7eOcNt0EYXfAhpyw8feBPQEKN3s5g0JrAs63nfoek-SLAvGKSyCZJLkTahLBTOiwmJE9z812J2nc9-qQV2toxXJvENzM2hP2ESf878Y-Fjbw9BOtok_5PvaHi_089RyDfpDIPgr4QYWFSb2hR4AkBqlYGDO2XDvMQRDZAEWtI2emhhyCebDF8WTgX2VEAwxcLFVxJLnETMSdcvEeFsfW2o9yFO3mK3ymm3Zgr3tQwBeqYXaJk6oRTpDU1EHtjk9lJByoHzZXfippv2hIJQ";

/// A *different* RS256 private key, used to forge a signature
/// the test JWKS will reject. Standalone fixture so the forged
/// path doesn't get a stale signature accidentally validated by
/// the legitimate keypair above.
pub(super) const FORGED_PRIVATE_PEM: &str = "-----BEGIN PRIVATE KEY-----\n\
MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQDLxOxukFcC7lIj\n\
vFggkGqkIJSgK8//VCuMKhTDyCCavJXIAYUM3T8r6IeIMK0h0LHZ2f8ZVEj/L6oQ\n\
DgPRqt5ALJYkGV9NGBZjiZb8AgbsBC3ZiPVuARVzxnTQmjDtFbf1f0vCr83Es/CW\n\
zhMRd0XRPR0JZsCfAWH1J3rpjEFZNiV8h5fm+bHhYYXhp1wYdjqYmrUYJlYJXDML\n\
OAUI+GuOx8TDE69ac5lJBnUm6X4BB2DaytDk+w617Ovz9pexKD7TVYyRCSft/6aa\n\
kWX+IXVVd83TW3hPRGTtmkGbyHB2Ej73W0v+qPn0noVTm7FYl+g2btyNypdpKaDD\n\
RnmFnqNhAgMBAAECggEAAsDsX4GelWTuUfqOd9EvybxBeekhqE4FQSGD8pHao/Cq\n\
tv9TQpVeUEv2AeiDfG6fKqYcIQXfLyigHBOuaEfOdVBM7puzqp1p1wpB0rG90XRs\n\
gLEtvODxnuUGiNeek+OcXupLlivNRGxYktch0ZV6qW1RnkIH1hY9EkSob+3D585R\n\
9K5ZgahXOeH8IePbjdMU3wbP4LkTo7IXZGMiJA8j77WR7DzV5JpTuUSRkGE2H7Qk\n\
u0oiBUaZbFT2ztF6ul5OdXZbmUzcIkYJU3E+uzaUhrtVfwCfDBNWvkOqL1h0hGuU\n\
riSzrWs07CyvEOGrk43L6ZN5mAQ/V8/8QmpLPedquQKBgQDwrtTCcrY+BdYpL8jf\n\
jzSWxfHuQB1pFb5zKKh4BDM3o+2UjMLHBmWwbi668pAAgcSJJRwmVKeKpv4dusRq\n\
d51V2tZ4nOjMwCj5/3Uc/Juqq/PsBolb9PLNMq7FOJsH3S+Mk0J/zGUz4ksPL+98\n\
txLeBn8XilhPwv79hsaz47WvmQKBgQDYvLM/06Aqn9QOgPLF57REuf/+FKR6+5Ip\n\
ZgpJEddg+29zSzwFbtZZ/rXPrqa9qDsFQZdOoeyf778trTUNzBY3Hscb22xvOvEO\n\
9xVxKVKz/TPkA+ze6BBjtJZMubX1HfhPrxsytLy4MeAOdK+mfhL6iuEDtMJatbeS\n\
B58gO22PCQKBgDhF+FLadUe9H5yTopi6p+YUtAMrlHTMc7IDMJiXCs6YkmToIGZe\n\
VYpRyLVHH2ou7R/PwGwp4N5nOwUCdQgbnXrEZt7eeQPebfY9x0kWuuLFv4tQ3+7T\n\
L63QitJr8Lt++K4ahDLTPFpML5aGc60qNMwaor6DRzCm++2VBIJs3D8JAoGBAMsV\n\
BWBpC0rlN+3fJZwK3/8Fybhp3zTdRLdFxZ1x+j4FWwjNFhCBKpho8jMHk3VijOr/\n\
6qbjUrUKEDjccznaYXaEgEy57YDL2dQL8St3bOb5+gVNKEY1bCYAsFR0Lureii0g\n\
Bnwcnjh5g1gIPg3jVCUuvGiclwAoBTnvqkqpZJ8BAoGBAM8dwSTMfzWA7lEPOcJz\n\
NeSvCuzg7foNX7q+yXwMotLDldg23OZuhiBHr38R58xJ3CNywB4LGCPVp6ZfDKm2\n\
CwhL3HB69b0jJHwsce2JcwdbIvIv+2OdwT6OOnM5w4YiZrFArgDoX5Fo2M24fFKH\n\
Tz/WdtE8/KPWoOHX2HhZncBt\n\
-----END PRIVATE KEY-----\n";

pub(super) const TEST_KID: &str = "test-kid";
pub(super) const TEST_SERVICE_ID: &str = "com.example.web";

pub(super) fn notif_encoding_key() -> EncodingKey {
    EncodingKey::from_rsa_pem(NOTIF_PRIVATE_PEM.as_bytes()).expect("RS256 private parses")
}

/// Build the DecodingKey from the JWK Apple would publish (`from_jwk`)
/// rather than going around it with a PEM shortcut. This keeps the
/// tests honest about the format the verifier consumes in production.
pub(super) fn notif_decoding_key_from_jwk() -> DecodingKey {
    let jwks_json = serde_json::json!({
        "keys": [{
            "kty": "RSA",
            "use": "sig",
            "alg": "RS256",
            "kid": TEST_KID,
            "n": NOTIF_PUBLIC_N,
            "e": "AQAB",
        }]
    });
    let jwks: jsonwebtoken::jwk::JwkSet =
        serde_json::from_value(jwks_json).expect("test JWKS is valid");
    let jwk = jwks.find(TEST_KID).expect("test kid present");
    DecodingKey::from_jwk(jwk).expect("DecodingKey builds from test JWK")
}

/// Sign an envelope claim set with [`NOTIF_PRIVATE_PEM`] (and the
/// [`TEST_KID`] header). Caller supplies the JSON payload so each
/// test can vary `iat`, `events`, etc.
pub(super) fn sign_envelope(payload: serde_json::Value) -> String {
    sign_envelope_with(
        &notif_encoding_key(),
        Some(TEST_KID),
        payload,
        Algorithm::RS256,
    )
}

pub(super) fn sign_envelope_with(
    key: &EncodingKey,
    kid: Option<&str>,
    payload: serde_json::Value,
    alg: Algorithm,
) -> String {
    let mut header = Header::new(alg);
    header.kid = kid.map(String::from);
    jsonwebtoken::encode(&header, &payload, key).expect("encode succeeds")
}

/// Build a well-formed `events` string for a `consent-revoked`
/// notification. Apple's wire format requires the `events` field
/// to be a JSON-encoded *string* (not a nested object) — pre-
/// stringify here so callers don't accidentally drop the layer.
pub(super) fn events_consent_revoked(sub: &str, event_time_ms: i64) -> String {
    serde_json::json!({
        "type": "consent-revoked",
        "sub": sub,
        "event_time": event_time_ms,
    })
    .to_string()
}

pub(super) fn envelope(iat: i64, exp: i64, events: &str) -> serde_json::Value {
    serde_json::json!({
        "iss": super::APPLE_ISSUER,
        "aud": TEST_SERVICE_ID,
        "iat": iat,
        "exp": exp,
        "jti": "test-jti",
        "events": events,
    })
}
