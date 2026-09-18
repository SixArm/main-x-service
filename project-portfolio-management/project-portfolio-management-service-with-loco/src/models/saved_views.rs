//! Saved-view model helpers (T-28l): the `ActiveModelBehavior` impl.
//! No custom finders needed — the controller queries directly, scoped
//! to the caller's own `sub`.

use loco_rs::prelude::*;

use super::_entities::saved_views;

impl ActiveModelBehavior for saved_views::ActiveModel {}
