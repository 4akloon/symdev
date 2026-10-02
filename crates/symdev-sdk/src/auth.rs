/// How requests to a source are authenticated.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Auth {
    #[default]
    None,
    /// AWS Signature V4 with the source's S3 keys (region `auto` for R2).
    S3,
}
