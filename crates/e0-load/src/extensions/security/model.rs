//! The declarations of the security extension (SCY-2 to SCY-4), as data types only.

use menura_security::{AttributeType, DatasetAccess, Role};

use crate::pointer::JsonPointer;
use crate::read::Location;

/// The `security_context` of the model (SCY-2), whose shape is valid (SCY-5.1.2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityContextDeclaration {
    /// The declared roles, in the order written, without repeats.
    pub roles: Vec<Role>,
    /// The declared attributes, in the order written, without a repeated name.
    pub attributes: Vec<Attribute>,
}

impl SecurityContextDeclaration {
    /// The type of the attribute named `name`, compared as an exact string (SCY-5.1.1).
    #[must_use]
    pub fn attribute(&self, name: &str) -> Option<AttributeType> {
        self.attributes
            .iter()
            .find(|attribute| attribute.name == name)
            .map(|attribute| attribute.attribute_type)
    }
}

/// One declared attribute of the security context (SCY-2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attribute {
    /// The name, as written.
    pub name: String,
    /// The type (SCY-2.2).
    pub attribute_type: AttributeType,
}

/// The `access` of one dataset, whose shape and roles are valid (SCY-5.1.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredAccess {
    /// The JSON Pointer of the dataset.
    pub dataset: JsonPointer,
    /// The JSON Pointer of the `data` member of the dataset's entry of ours, where every
    /// diagnostic about the access is located (E0C-2.6).
    pub data: JsonPointer,
    /// Where that `data` member starts in the document text.
    pub location: Location,
    /// The access.
    pub access: DatasetAccess,
}

/// One row policy of a dataset (SCY-4), whose shape and roles are valid (SCY-5.1.5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowPolicy {
    /// The JSON Pointer of the dataset.
    pub dataset: JsonPointer,
    /// The JSON Pointer of the `data` member of the dataset's entry of ours (E0C-2.6).
    pub data: JsonPointer,
    /// Where that `data` member starts in the document text.
    pub location: Location,
    /// The name, unique in the model.
    pub name: String,
    /// The text of `condition`, parsed and typed by E0.f (SCY-5.1.6).
    pub condition: String,
    /// The roles the policy grants rows to; `None` when absent: it restricts everyone.
    pub roles: Option<Vec<Role>>,
    /// The roles the policy does not apply to; `None` when absent.
    pub exempt_roles: Option<Vec<Role>>,
}

/// Every declaration of the security extension of a model.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Security {
    /// The JSON Pointer of the semantic model, where `W0C-40` is located (SCY-5.1.7).
    pub model: JsonPointer,
    /// The `security_context`, when the model has one.
    pub context: Option<SecurityContextDeclaration>,
    /// The `access` of every dataset that has one, in document order.
    pub accesses: Vec<DeclaredAccess>,
    /// Every row policy, in document order.
    pub policies: Vec<RowPolicy>,
}

impl Security {
    /// The declared access of the dataset at `dataset`, when it has one.
    #[must_use]
    pub fn access(&self, dataset: &JsonPointer) -> Option<&DeclaredAccess> {
        self.accesses
            .iter()
            .find(|declared| declared.dataset == *dataset)
    }
}
