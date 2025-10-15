use kovi::build_bot;

fn main() {
    let bot = build_bot!(kovi_plugin_cmd, feedback);
    bot.run();
}
