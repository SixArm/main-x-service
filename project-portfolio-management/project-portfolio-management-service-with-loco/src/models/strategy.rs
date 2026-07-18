//! Strategy model glue (PPM Phase C): `ActiveModelBehavior` impls
//! for the ideas / scenarios / objectives / objective-links /
//! benefits entities (finders live in the controller — these tables
//! have no cross-cutting query helpers yet).

use loco_rs::prelude::*;

use super::_entities::{benefits, ideas, objective_links, objectives, scenarios};

impl ActiveModelBehavior for ideas::ActiveModel {}
impl ActiveModelBehavior for scenarios::ActiveModel {}
impl ActiveModelBehavior for objectives::ActiveModel {}
impl ActiveModelBehavior for objective_links::ActiveModel {}
impl ActiveModelBehavior for benefits::ActiveModel {}
