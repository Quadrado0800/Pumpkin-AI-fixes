use std::{sync::Arc, time::Duration};

use pumpkin_core::{
    block::entities::chest::ChestBlockEntity,
    entity::{Entity, EntityBase, passive::cat::CatEntity},
    server::Server,
    world::BlockFlags,
};
use pumpkin_data::{Block, entity::EntityType};
use pumpkin_util::math::{position::BlockPos, vector3::Vector3};

use crate::bot::{HeadlessBot, MAX_POLL_TICKS};

pub async fn run_cat_chest_test(
    server: &Arc<Server>,
    bot: &mut HeadlessBot,
    summary_lines: &mut Vec<String>,
) -> Result<(), String> {
    let player = server.get_player_by_name(&bot.name).ok_or("missing bot")?;
    let world = player.world();
    let base = player.position();
    let chest_pos = BlockPos::new(
        base.x.floor() as i32 + 2,
        base.y.floor() as i32,
        base.z.floor() as i32,
    );

    world.set_block_state(
        &chest_pos,
        Block::CHEST.default_state.id,
        BlockFlags::NOTIFY_ALL,
    );
    world.add_block_entity(Arc::new(ChestBlockEntity::new(chest_pos)));

    let cat = CatEntity::new(Entity::new(
        world.clone(),
        Vector3::new(
            f64::from(chest_pos.0.x) + 0.5,
            f64::from(chest_pos.0.y) + 1.0,
            f64::from(chest_pos.0.z) + 0.5,
        ),
        &EntityType::CAT,
    ));
    cat.set_sitting(true);
    let cat_entity: Arc<dyn EntityBase> = cat.clone();
    if !world.spawn_entity(cat_entity) {
        return Err("failed to spawn the sitting cat".to_string());
    }

    let before = player
        .current_screen_handler
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .clone();
    let screen_changed = || {
        !Arc::ptr_eq(
            &before,
            &player
                .current_screen_handler
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        )
    };

    bot.use_block(chest_pos, 1).await?;
    for _ in 0..MAX_POLL_TICKS {
        tokio::time::sleep(Duration::from_millis(5)).await;
        server.tick();
        if screen_changed() {
            break;
        }
    }

    if screen_changed() {
        player.close_handled_screen();
        cat.get_entity().remove();
        return Err("a sitting cat did not block the chest".to_string());
    }

    cat.set_sitting(false);
    bot.use_block(chest_pos, 2).await?;
    let mut opened = false;
    for _ in 0..MAX_POLL_TICKS {
        tokio::time::sleep(Duration::from_millis(5)).await;
        server.tick();
        if screen_changed() {
            opened = true;
            break;
        }
    }

    player.close_handled_screen();
    cat.get_entity().remove();
    world.set_block_state(
        &chest_pos,
        Block::AIR.default_state.id,
        BlockFlags::NOTIFY_ALL,
    );

    if !opened {
        return Err("a standing cat still blocked the chest".to_string());
    }

    summary_lines.push("[PASS] bot:sitting_cat_blocks_chest".to_string());
    Ok(())
}
