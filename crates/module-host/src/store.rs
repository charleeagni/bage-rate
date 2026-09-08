//! The Store vocabulary a Module may name.
//!
//! Everything here is a SeaORM query or write trait re-exported so that a
//! Module's own source never says `sea_orm`. There is no connection in this
//! module by design: a Module reaches the Store only through the handle
//! [`ModuleCtx`](crate::ModuleCtx) hands it, so every read and write is
//! inside a borrowed connection or a borrowed transaction.

pub use sea_orm::{
    ActiveModelBehavior, ActiveModelTrait, ActiveValue, ColumnTrait, Condition, ConnectionTrait,
    DbErr, EntityTrait, IntoActiveModel, ModelTrait, Order, PaginatorTrait, QueryFilter,
    QueryOrder, QuerySelect, QueryTrait, TransactionTrait,
};
pub use sea_orm::{ActiveValue::NotSet, ActiveValue::Set, ActiveValue::Unchanged};

/// The Store connection a Module borrows for reads outside a transaction.
pub type Store = sea_orm::DatabaseConnection;

/// The open transaction a Module's atomic work runs inside. It is a Store
/// connection for query purposes, so every SeaORM read and write accepts it.
pub type Txn = sea_orm::DatabaseTransaction;
