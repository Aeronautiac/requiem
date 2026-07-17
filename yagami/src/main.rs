// likely need a unified receiver structure which can handle websockets + normal sockets, or just
// use websockets regardless of the context
//
// no need for an "accounts" concept right now. simply just give keys out for doing certain actions,
// and expose different game joining variants.
//
// for now just have a "closed" variant where you can create a game, get given an admin key, and log
// into the admin using only that key. the admin can add players (and give a key to different people).
//
// later, introduce an account system + verification for limiting things like active games from one person.
// after this is added, it would be possible to have open games where you can kinda trust that one person only has one character.
//
// the important thing right now is getting things working and running a prototype game
//
// PLAN:
// - database stores active games, and active games have a set of keys which lead to a set of actors
// (and their creation sequence number if necessary for things like rewind). it also stores that
// game's action sequence, and a set of commands accumulated for every actor.
// also stores things like messages, notebook events, etc...
// - for game hosting, its basically just more tokio stuff (almost exactly the same as in armonia's
// tauri backend), just scaled up to multiple instances
// - communicate directly with sub-processes via pipes. nothing more complicated is needed. KISS
//
// the standard flow:
// - client connects to the server
// - client either requests a game creation or a game join (using a key)
// - game creation returns an admin key to the client
// - game join returns an actor set + a command sequence for all those actors
// - while the game runs, the server sends command batches to every client to keep it up to date
// (the server does null ticks every 5 or so seconds if at least one client is connected). it also
// sends out commands if say another client performed an action.
// - clients request actions and receive responses
//
// this server will be pretty damn easy to write
//

// the main function should just be an entrypoint into the server loop
fn main() {
    println!("Hello, world!");
}
