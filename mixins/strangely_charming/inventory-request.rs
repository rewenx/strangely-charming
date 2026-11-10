fn hook(
    game: &mut crate::game::Game<'_>,
    host: &mut LanHost,
    conn: u32,
    packet: &S,
) -> core::ops::ControlFlow<()> {
    use crate::game::strangely_charming::{ACTION_CHANNEL, PORTABLE_TABLE, carries_table, compact};
    if let S::CustomClickAction(p)=packet && let Some(a)=p.id.strip_prefix(crate::game::sc_restoration::CHANNEL){
        if a.len()<=24 && let Some(gi)=host.hub.guests.iter().position(|g|g.conn==conn && g.placed && !g.dead && !g.spectator() && g.health>0.) {
            if a.ends_with("/yes") && let Some(Some((lo,hi)))=game.as_guest(host,conn,|g|g.sc_offer_bounds()){
                for x in lo[0]..=hi[0] {for z in lo[2]..=hi[2] {let p=[x,lo[1],z]; if game.under_spawn_protection(host,gi,p){game.spawn_protection_message(host,conn,p);return core::ops::ControlFlow::Break(())}}}
            }
            game.as_guest(host,conn,|g|g.sc_restore_answer(a));
        }
        return core::ops::ControlFlow::Break(())
    }
    if let S::CustomClickAction(p)=packet && p.id=="strangely_charming:share_item" {game.sc_share_packet(host,conn,p.payload.as_ref());return core::ops::ControlFlow::Break(());}
    if let S::PlayerAction(action) = packet
        && action.action == 2
        && game.sc_rune_at(action.pos)
        && let Some(play) = host.play.get_mut(&conn)
    {
        play.destroy_pos = None;
        play.delayed = None;
    }
    if let S::CustomClickAction(p) = packet
        && let Some(action) = p.id.strip_prefix(crate::game::sc_travel::CHANNEL)
    {
        if action.len() <= 160 && host.hub.guests.iter().any(|g| g.conn == conn && g.placed && !g.dead && !g.spectator() && g.health > 0.) {
            // The active host is borrowed out of Game while it handles guest packets.
            if matches!(action.split('/').nth(2), Some("build" | "build_diagram")) {
                let Some(horizon_world::nbt::Tag::IntArray(v))=p.payload.as_ref() else {return core::ops::ControlFlow::Break(())};
                let [x,y,z,axis,_]=v.as_slice() else{return core::ops::ControlFlow::Break(())};
                if !matches!(axis,0|2) || [x,y,z].iter().any(|v|v.unsigned_abs()>30_000_000) {return core::ops::ControlFlow::Break(())}
                let gi=host.hub.guests.iter().position(|g|g.conn==conn).unwrap();
                for h in -2..=2i32 {for dy in 0..=4 {if h.abs()==2 && (dy==0 || dy==4){continue;}
                    let mut cell=[*x,*y+dy,*z];cell[*axis as usize]+=h;
                    if game.under_spawn_protection(host,gi,cell){game.spawn_protection_message(host,conn,cell);return core::ops::ControlFlow::Break(())}
                }}
            }
            game.as_guest(host, conn, |g| g.sc_travel_action(action, p.payload.as_ref()));
        }
        return core::ops::ControlFlow::Break(());
    }
    if let S::CustomClickAction(p) = packet
        && let Some(action) = p.id.strip_prefix("strangely_charming:quest/")
    {
        if action.len() <= 80
            && host
                .hub
                .guests
                .iter()
                .any(|g| g.conn == conn && g.placed && !g.dead && !g.spectator() && g.health > 0.)
        {
            game.as_guest(host, conn, |g| g.sc_quest_action(action));
        }
        return core::ops::ControlFlow::Break(());
    }
    let S::CustomPayload(p) = packet else {
        return core::ops::ControlFlow::Continue(());
    };
    if p.channel == crate::game::sc_grave_hud::CHANNEL {
        game.as_guest(host,conn,|g|g.sc_answer_grave_inspection(&p.data));
        return core::ops::ControlFlow::Break(());
    }
    if p.channel != ACTION_CHANNEL {
        return core::ops::ControlFlow::Continue(());
    }
    let done = core::ops::ControlFlow::Break(());
    let [action, a, b, c, d] = p.data.as_slice() else {
        return done;
    };
    let window = i32::from_be_bytes([*a, *b, *c, *d]);
    let Some(i) = host.hub.guests.iter().position(|g| g.conn == conn) else {
        return done;
    };
    let guest = &host.hub.guests[i];
    if guest.dead || guest.spectator() || guest.health <= 0. {
        return done;
    }
    if *action == 3 {
        game.sc_lan_cycle(host, conn, window);
        return done;
    }
    if *action == 1
        && window == 0
        && guest.menu.is_none()
        && carries_table(guest)
        && horizon_mods::evanders_config::enabled("portable_crafting")
    {
        game.lan_open_crafting(host, conn, PORTABLE_TABLE);
    } else if *action == 2 && horizon_mods::evanders_config::enabled("inventory_tidying") {
        if let Some(menu) = guest.menu.as_ref() {
            if window != menu.window || !menu.slots.still_valid(&game.world, guest, menu.pos) {
                return done;
            }
            let slots = menu.slots;
            let supported = match slots {
                Slots::Double(..) | Slots::Ender => true,
                Slots::Block((x, y, z)) => {
                    let block = game.block_name([x, y, z]);
                    matches!(
                        block.as_ref(),
                        "minecraft:chest" | "minecraft:barrel" | "minecraft:trapped_chest"
                    ) || block.ends_with("shulker_box")
                }
                _ => false,
            };
            if supported && let Some(mut items) = slots.read(&game.world, guest) {
                compact(&mut items, |s| game.max_stack(s));
                slots.write(&mut game.world, &mut host.hub.guests[i], items);
                host.hub.sync_containers(&game.world);
            }
        } else if window == 0 {
            let inventory = &mut host.hub.guests[i].inventory;
            if let Some(bag) = inventory.get_mut(9..36) {
                compact(bag, |s| game.max_stack(s));
                host.hub.send_inventory(i);
            }
        }
    }
    done
}
