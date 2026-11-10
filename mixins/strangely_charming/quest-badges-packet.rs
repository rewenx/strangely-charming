fn hook(game: &mut crate::game::Game<'_>, packet: &ClientboundPlayPacket) {
    if let ClientboundPlayPacket::CustomPayload(p) = packet
        && p.channel == crate::game::sc_travel::PREVIEW_DIAGRAM
        && p.data.len() == 32
        && let Ok(id) = std::str::from_utf8(&p.data)
    {
        game.sc_preview_diagram(id);
        return;
    }
    if let ClientboundPlayPacket::CustomPayload(p) = packet
        && p.channel == crate::game::sc_runes::FORGET_FRAME {
        crate::game::sc_runes::receive_forget_frame(&p.data);
        return;
    }
    if let ClientboundPlayPacket::CustomPayload(p) = packet
        && p.channel == crate::game::sc_runes::SURVEY
    {
        crate::game::sc_runes::receive_survey(&p.data);
        return;
    }
    if let ClientboundPlayPacket::CustomPayload(p) = packet
        && p.channel == crate::game::sc_runes::VIEW
    {
        crate::game::sc_runes::receive(&p.data);
        return;
    }
    if let ClientboundPlayPacket::CustomPayload(p) = packet
        && p.channel == crate::game::sc_grave_hud::CHANNEL
    {
        game.sc_read_grave_view(&p.data);
        return;
    }
    if let ClientboundPlayPacket::CustomPayload(p) = packet
        && p.channel == crate::game::sc_journal::QUEST_VIEW
    {
        game.sc_receive_quest_preview(&p.data);
        return;
    }
    if let ClientboundPlayPacket::CustomPayload(p) = packet
        && p.channel == crate::game::sc_journal::VIEW
    {
        game.sc_receive_journal(&p.data);
    }
    if let ClientboundPlayPacket::CustomPayload(p) = packet
        && p.channel == crate::game::stations::sc_trade_cycle::CAP
    {
        game.sc_receive_cycle_cap(&p.data);
    }
    if let ClientboundPlayPacket::CustomPayload(p) = packet
        && p.channel == crate::game::sc_alerts::CHANNEL
    {
        game.sc_receive_alert(&p.data);
    }
    if let ClientboundPlayPacket::CustomPayload(p) = packet
        && p.channel == crate::game::sc_quests::BADGES
    {
        crate::game::sc_quests::read_badges(&p.data);
    }
}
