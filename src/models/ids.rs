use serde::Serialize;
use std::fmt;

macro_rules! entity_id {
    ($(#[$meta:meta])* $name:ident) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize)]
        #[serde(transparent)]
        pub struct $name(i64);

        impl $name {
            /// Constructs an ID assigned by persistence or tests.
            pub const fn from_i64(id: i64) -> Self {
                Self(id)
            }

            pub const fn as_i64(self) -> i64 {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }

        impl From<$name> for i64 {
            fn from(id: $name) -> Self {
                id.0
            }
        }

        impl PartialEq<i64> for $name {
            fn eq(&self, other: &i64) -> bool {
                self.0 == *other
            }
        }
    };
}

entity_id!(
    /// Stable row ID of a task.
    TaskId
);
entity_id!(
    /// Stable row ID of a TODO (not the task-scoped index shown in the CLI).
    TodoId
);
entity_id!(
    /// Task-scoped sequential TODO number (`track todo done 3`).
    TodoIndex
);
entity_id!(
    /// Stable row ID of a link.
    LinkId
);
entity_id!(
    /// Task-scoped sequential link number (`track link delete 2`).
    LinkIndex
);
entity_id!(
    /// Stable row ID of a scrap.
    ScrapId
);
entity_id!(
    /// Task-scoped sequential scrap number.
    ScrapIndex
);
entity_id!(
    /// Stable row ID of a worktree record.
    WorktreeId
);
entity_id!(
    /// Stable row ID of a remote repo link on a worktree.
    RepoLinkId
);
entity_id!(
    /// Stable row ID of a task repository registration.
    TaskRepoId
);
entity_id!(
    /// Task-scoped sequential repository number (`track repo remove 1`).
    RepoIndex
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_distinct_types() {
        let task = TaskId::from_i64(3);
        let index = TodoIndex::from_i64(3);
        assert_eq!(task.as_i64(), index.as_i64());
        assert_eq!(task.to_string(), "3");
    }

    #[test]
    fn ids_serialize_as_numbers() {
        let json = serde_json::to_value(TodoIndex::from_i64(4)).unwrap();
        assert_eq!(json, 4);
    }
}
