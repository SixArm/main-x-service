//! Geographic boundary mappings for postcodes
//!
//! Maps postcodes to health and social care boundaries including
//! Local Authority, Integrated Care Board (ICB), LSOA, and others.
//! Aligned with NHS ODS postcode CodeSystem.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Geographic boundary mappings for a postcode
///
/// Contains various geographic values/boundaries for a postcode including
/// which Lower Super Output Area, Local Authority, Parliamentary Constituency,
/// and Integrated Care Board boundary a postcode falls within.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PostcodeGeography {
    /// The postcode (e.g. "SW1A 1AA")
    pub postcode: String,

    /// 2011 Lower Super Output Area code (e.g. "E01004736")
    pub lsoa11: Option<String>,

    /// Local Authority code (e.g. "E09000033")
    pub local_authority: Option<String>,

    /// Local Authority name (e.g. "Westminster")
    pub local_authority_name: Option<String>,

    /// Integrated Care Board code
    pub icb: Option<String>,

    /// Integrated Care Board name
    pub icb_name: Option<String>,

    /// NHS England Region code
    pub nhs_england_region: Option<String>,

    /// NHS England Region name
    pub nhs_england_region_name: Option<String>,

    /// Parliamentary Constituency code
    pub parliamentary_constituency: Option<String>,

    /// Parliamentary Constituency name
    pub parliamentary_constituency_name: Option<String>,

    /// Government Office Region code
    pub government_office_region: Option<String>,

    /// Cancer Alliance code
    pub cancer_alliance: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_postcode_geography_construction() {
        let geo = PostcodeGeography {
            postcode: "SW1A 1AA".to_string(),
            lsoa11: Some("E01004736".to_string()),
            local_authority: Some("E09000033".to_string()),
            local_authority_name: Some("Westminster".to_string()),
            icb: Some("QWE".to_string()),
            icb_name: Some("NHS North West London ICB".to_string()),
            nhs_england_region: Some("Y56".to_string()),
            nhs_england_region_name: Some("London".to_string()),
            parliamentary_constituency: Some("E14000639".to_string()),
            parliamentary_constituency_name: Some("Cities of London and Westminster".to_string()),
            government_office_region: Some("E12000007".to_string()),
            cancer_alliance: None,
        };
        assert_eq!(geo.postcode, "SW1A 1AA");
        assert_eq!(geo.local_authority_name.as_deref(), Some("Westminster"));
    }

    #[test]
    fn test_postcode_geography_serialization() {
        let geo = PostcodeGeography {
            postcode: "LS1 4AP".to_string(),
            lsoa11: Some("E01011229".to_string()),
            local_authority: Some("E08000035".to_string()),
            local_authority_name: Some("Leeds".to_string()),
            icb: Some("X2C4Y".to_string()),
            icb_name: Some("NHS West Yorkshire ICB".to_string()),
            nhs_england_region: None,
            nhs_england_region_name: None,
            parliamentary_constituency: None,
            parliamentary_constituency_name: None,
            government_office_region: None,
            cancer_alliance: None,
        };
        let json = serde_json::to_string(&geo).unwrap();
        let deser: PostcodeGeography = serde_json::from_str(&json).unwrap();
        assert_eq!(deser.postcode, "LS1 4AP");
        assert_eq!(deser.icb_name.as_deref(), Some("NHS West Yorkshire ICB"));
    }
}
