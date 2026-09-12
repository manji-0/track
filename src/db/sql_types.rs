//! rusqlite conversions for domain identifiers (persistence adapter).

use crate::models::{
    HttpUrl, LinkId, LinkIndex, RepoIndex, RepoLinkId, ScrapId, ScrapIndex, TaskAlias, TaskId,
    TaskRepoId, TicketId, TodoId, TodoIndex, WorktreeId,
};
use rusqlite::types::{FromSql, FromSqlResult, ToSql, ToSqlOutput, ValueRef};

macro_rules! impl_i64_sql {
    ($name:ty) => {
        impl ToSql for $name {
            fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
                Ok(ToSqlOutput::from(self.as_i64()))
            }
        }

        impl FromSql for $name {
            fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
                i64::column_result(value).map(Self::from_i64)
            }
        }
    };
}

impl_i64_sql!(TaskId);
impl_i64_sql!(TodoId);
impl_i64_sql!(TodoIndex);
impl_i64_sql!(LinkId);
impl_i64_sql!(LinkIndex);
impl_i64_sql!(ScrapId);
impl_i64_sql!(ScrapIndex);
impl_i64_sql!(WorktreeId);
impl_i64_sql!(RepoLinkId);
impl_i64_sql!(TaskRepoId);
impl_i64_sql!(RepoIndex);

impl ToSql for TicketId {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        self.as_str().to_sql()
    }
}

impl FromSql for TicketId {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        String::column_result(value).map(TicketId::from_stored)
    }
}

impl ToSql for TaskAlias {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        self.as_str().to_sql()
    }
}

impl FromSql for TaskAlias {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        String::column_result(value).map(TaskAlias::from_stored)
    }
}

impl ToSql for HttpUrl {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        self.as_str().to_sql()
    }
}

impl FromSql for HttpUrl {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        String::column_result(value).map(HttpUrl::from_stored)
    }
}
