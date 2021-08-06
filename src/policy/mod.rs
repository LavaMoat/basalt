//! Types for LavaMoat policy files.

use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::collections::BTreeMap;
use std::fmt;

pub mod builder;

/// Trait for the merge operation.
pub trait Merge {
    /// Apply overrides from `from`
    fn merge(&mut self, from: &Self);
}

/// LavaMoat policy file.
#[derive(Serialize, Deserialize, Default, Debug, Eq, PartialEq)]
pub struct Policy {
    /// Collection of package resources for the policy.
    pub resources: BTreeMap<String, PackagePolicy>,
}

impl Policy {
    /// Insert a policy into the package resources.
    pub fn insert<S: AsRef<str>>(&mut self, key: S, value: PackagePolicy) {
        self.resources.insert(key.as_ref().into(), value);
    }
}

impl Merge for Policy {
    fn merge(&mut self, from: &Self) {
        for (k, v) in from.resources.iter() {
            if let Some(pkg) = self.resources.get_mut(k) {
                pkg.merge(v);
            } else {
                self.resources.insert(k.to_string(), v.clone());
            }
        }
    }
}

/// Policy for a single package.
#[derive(Serialize, Deserialize, Clone, Default, Debug, Eq, PartialEq)]
#[serde(default)]
pub struct PackagePolicy {
    /// Does this policy allow native bindings.
    #[serde(skip_serializing_if = "is_false")]
    pub native: bool,
    /// Determine how to treat the environment when hardening.
    #[serde(skip_serializing_if = "EnvPolicy::is_default")]
    pub env: EnvPolicy,
    /// Policy for builtin packages.
    #[serde(skip_serializing_if = "PolicyGroup::is_empty")]
    pub builtin: PolicyGroup,
    /// Policy for the ambient authority provided by globals.
    #[serde(skip_serializing_if = "PolicyGroup::is_empty")]
    pub globals: PolicyGroup,
    /// Collection of packages accessible via this package.
    #[serde(skip_serializing_if = "PolicyGroup::is_empty")]
    pub packages: PolicyGroup,
}

impl PackagePolicy {
    /// Determine if this policy has no builtins, globals and packages.
    pub fn is_empty(&self) -> bool {
        self.builtin.map.is_empty()
            && self.globals.map.is_empty()
            && self.packages.map.is_empty()
    }
}

impl Merge for PackagePolicy {
    fn merge(&mut self, from: &Self) {
        self.native = from.native;
        self.env = from.env;
        self.builtin.merge(&from.builtin);
        self.globals.merge(&from.globals);
        self.packages.merge(&from.packages);
    }
}

/// Represents a code access permission for a package policy entry.
///
/// Currently this is just a boolean switch but later we may
/// modify this to represent [read, write, execute] permissions.
#[derive(Copy, Clone, Default, Debug, Eq, PartialEq)]
pub struct PolicyAccess {
    flag: bool,
}

impl Serialize for PolicyAccess {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bool(self.flag)
    }
}

impl<'de> Deserialize<'de> for PolicyAccess {
    fn deserialize<D>(deserializer: D) -> Result<PolicyAccess, D::Error>
    where
        D: Deserializer<'de>,
    {
        let flag = deserializer.deserialize_bool(PolicyAccessVisitor)?;
        Ok(PolicyAccess { flag })
    }
}

struct PolicyAccessVisitor;

impl<'de> Visitor<'de> for PolicyAccessVisitor {
    type Value = bool;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter
            .write_str("a boolean is required for policy access permissions")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(value)
    }
}

impl From<bool> for PolicyAccess {
    fn from(value: bool) -> Self {
        PolicyAccess { flag: value }
    }
}

/// Encapsulates a map from code access to permission flag.
#[derive(Serialize, Deserialize, Clone, Default, Debug, Eq, PartialEq)]
pub struct PolicyGroup {
    #[serde(flatten)]
    map: BTreeMap<String, PolicyAccess>,
}

impl PolicyGroup {
    /// Determine if this policy map is empty.
    pub fn is_empty(policy_map: &PolicyGroup) -> bool {
        policy_map.map.is_empty()
    }

    /// Insert a permission into the policy.
    pub fn insert<S: AsRef<str>>(&mut self, key: S, value: PolicyAccess) {
        self.map.insert(key.as_ref().into(), value);
    }

    /// Append a map of packages to this group.
    pub fn append(&mut self, other: &mut BTreeMap<String, PolicyAccess>) {
        self.map.append(other);
    }
}

impl Merge for PolicyGroup {
    fn merge(&mut self, from: &Self) {
        for (k, v) in from.map.iter() {
            self.map.insert(k.to_string(), *v);
        }
    }
}

/// Enumeration of policy values for the environment access.
#[derive(Serialize, Deserialize, Eq, PartialEq, Copy, Clone, Debug)]
pub enum EnvPolicy {
    /// Freeze the environment.
    #[serde(rename = "frozen")]
    Frozen,
    /// Do not freeze the environment.
    #[serde(rename = "unfrozen")]
    Unfrozen,
}

impl EnvPolicy {
    /// Determine if a policy is equal to the default policy.
    pub fn is_default(policy: &EnvPolicy) -> bool {
        policy == &Default::default()
    }
}

impl Default for EnvPolicy {
    fn default() -> Self {
        EnvPolicy::Frozen
    }
}

// So we can ignore false booleans from serialization.
fn is_false(flag: &bool) -> bool {
    flag == &false
}
