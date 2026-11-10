fn hook(commands: &mut Vec<NativeCommand>) {
    commands.extend([
        NativeCommand::op("scrunes unlock-all", 2),
        NativeCommand::op("scrunes unlock-all <player:player_name>", 2),
        NativeCommand::op("scgraves list", 2),
        NativeCommand::op("scgraves test-break", 2),
        NativeCommand::op("scgraves simulate-death", 2),
        NativeCommand::op("scgraves simulate-death <player:player_name>", 2),
        NativeCommand::op("scgraves simulate-loop", 2),
        NativeCommand::op("scgraves simulate-loop <player:player_name>", 2),
        NativeCommand::op("scgraves reopen <death:word> confirmed", 2),
        NativeCommand::op("scgraves spawn <death:word>", 2),
    ]);
}
