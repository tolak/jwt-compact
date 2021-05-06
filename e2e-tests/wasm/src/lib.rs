//! Module testing that the library can verify JWTs in WASM.

#![no_std]

extern crate alloc;

use chrono::Duration;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use alloc::string::{String, ToString};
use core::{convert::TryFrom, fmt};

use jwt_compact::{
    alg::{Ed25519, Hs256, Hs384, Hs512, RSAPrivateKey, RSAPublicKey, Rsa},
    jwk::{JsonWebKey, JwkError},
    Algorithm, AlgorithmExt, Claims, Header, TimeOptions, Token, UntrustedToken,
};

#[wasm_bindgen]
extern "C" {
    type Error;

    #[wasm_bindgen(constructor)]
    fn new(message: &str) -> Error;
}

/// Sample token claims.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SampleClaims {
    #[serde(rename = "sub")]
    subject: String,
    name: String,
    #[serde(default)]
    admin: bool,
}

/// Converts type to a JS `Error`.
fn to_js_error(e: impl fmt::Display) -> Error {
    Error::new(&e.to_string())
}

fn extract_claims(token: &Token<SampleClaims>) -> Result<&SampleClaims, JsValue> {
    Ok(&token
        .claims()
        .validate_expiration(&TimeOptions::default())
        .map_err(to_js_error)?
        .custom)
}

fn do_verify_token<T, J>(token: &UntrustedToken, jwk: J) -> Result<JsValue, JsValue>
where
    T: Algorithm + Default,
    T::VerifyingKey: TryFrom<J, Error = JwkError>,
{
    let verifying_key = <T::VerifyingKey>::try_from(jwk).map_err(to_js_error)?;

    let token = T::default()
        .validate_integrity::<SampleClaims>(token, &verifying_key)
        .map_err(to_js_error)?;
    let claims = extract_claims(&token)?;
    Ok(JsValue::from_serde(claims).expect("Cannot serialize claims"))
}

fn do_create_token<T, J>(claims: SampleClaims, jwk: J) -> Result<String, JsValue>
where
    T: Algorithm + Default,
    T::SigningKey: TryFrom<J, Error = JwkError>,
{
    let secret_key = <T::SigningKey>::try_from(jwk).map_err(to_js_error)?;
    let claims = Claims::new(claims).set_duration(&TimeOptions::default(), Duration::hours(1));

    let token = T::default()
        .token(Header::default(), &claims, &secret_key)
        .map_err(to_js_error)?;
    Ok(token)
}

#[wasm_bindgen(js_name = "verifyHashToken")]
pub fn verify_hash_token(token: &str, secret_key: &JsValue) -> Result<JsValue, JsValue> {
    let token = UntrustedToken::new(token).map_err(to_js_error)?;
    let jwk: JsonWebKey<'_> = secret_key.into_serde().map_err(to_js_error)?;

    match token.algorithm() {
        "HS256" => do_verify_token::<Hs256, _>(&token, &jwk),
        "HS384" => do_verify_token::<Hs384, _>(&token, &jwk),
        "HS512" => do_verify_token::<Hs512, _>(&token, &jwk),
        _ => Err(to_js_error("Invalid algorithm").into()),
    }
}

#[wasm_bindgen(js_name = "createHashToken")]
pub fn create_hash_token(
    claims: &JsValue,
    secret_key: &JsValue,
    alg: &str,
) -> Result<String, JsValue> {
    let jwk: JsonWebKey<'_> = secret_key.into_serde().map_err(to_js_error)?;
    let claims: SampleClaims = claims.into_serde().map_err(to_js_error)?;
    match alg {
        "HS256" => do_create_token::<Hs256, _>(claims, &jwk),
        "HS384" => do_create_token::<Hs384, _>(claims, &jwk),
        "HS512" => do_create_token::<Hs512, _>(claims, &jwk),
        _ => Err(to_js_error("Invalid algorithm").into()),
    }
}

#[wasm_bindgen(js_name = "verifyRsaToken")]
pub fn verify_rsa_token(token: &str, public_key_pem: &str) -> Result<JsValue, JsValue> {
    let public_key = pem::parse(public_key_pem).map_err(to_js_error)?.contents;
    let public_key = RSAPublicKey::from_pkcs8(&public_key).map_err(to_js_error)?;
    let token = UntrustedToken::new(token).map_err(to_js_error)?;

    let rsa = Rsa::with_name(token.algorithm());
    let token = rsa
        .validate_integrity::<SampleClaims>(&token, &public_key)
        .map_err(to_js_error)?;
    let claims = extract_claims(&token)?;
    Ok(JsValue::from_serde(claims).expect("Cannot serialize claims"))
}

#[wasm_bindgen(js_name = "createRsaToken")]
pub fn create_rsa_token(
    claims: &JsValue,
    private_key_pem: &str,
    alg: &str,
) -> Result<String, JsValue> {
    let private_key = pem::parse(private_key_pem).map_err(to_js_error)?.contents;
    let private_key = RSAPrivateKey::from_pkcs8(&private_key).map_err(to_js_error)?;

    let claims: SampleClaims = claims.into_serde().map_err(to_js_error)?;
    let claims = Claims::new(claims).set_duration(&TimeOptions::default(), Duration::hours(1));

    let token = Rsa::with_name(alg)
        .token(Header::default(), &claims, &private_key)
        .map_err(to_js_error)?;
    Ok(token)
}

#[wasm_bindgen(js_name = "verifyEdToken")]
pub fn verify_ed_token(token: &str, public_key: &JsValue) -> Result<JsValue, JsValue> {
    let jwk: JsonWebKey<'_> = public_key.into_serde().map_err(to_js_error)?;
    let token = UntrustedToken::new(token).map_err(to_js_error)?;
    do_verify_token::<Ed25519, _>(&token, &jwk)
}

#[wasm_bindgen(js_name = "createEdToken")]
pub fn create_ed_token(claims: &JsValue, private_key: &JsValue) -> Result<String, JsValue> {
    let jwk: JsonWebKey<'_> = private_key.into_serde().map_err(to_js_error)?;
    let claims: SampleClaims = claims.into_serde().map_err(to_js_error)?;
    do_create_token::<Ed25519, _>(claims, &jwk)
}
