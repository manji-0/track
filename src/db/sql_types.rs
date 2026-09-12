//! rusqlite conversions for domain identifiers (persistence adapter).

use crate::models::{TaskId, TicketId, TodoId, TodoIndex};
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
