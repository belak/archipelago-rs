use std::{collections::HashMap, ops::BitOr};

use serde::{ser::SerializeMap, Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

/// Server -> Client messages
///
/// These packets are are sent from the multiworld server to the client. They
/// are not messages which the server accepts.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "cmd")]
pub enum ServerMessage {
    ReceivedItems(ReceivedItems),
    LocationInfo(LocationInfo),
    RoomUpdate(RoomUpdate),
    PrintJSON(PrintJSON),
    Bounced(Bounced),
    Retrieved(Retrieved),
    SetReply(SetReply),

    InvalidPacket(InvalidPacket),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "cmd")]
pub enum AnonymousServerMessage {
    RoomInfo(RoomInfo),
    ConnectionRefused(ConnectionRefused),
    Connected(Connected),
    DataPackage(DataPackage),

    InvalidPacket(InvalidPacket),
}

/// Sent to clients when they connect to an Archipelago server.
#[derive(Debug, Serialize, Deserialize)]
pub struct RoomInfo {
    /// Object denoting the version of Archipelago which the server is running.
    pub version: NetworkVersion,

    /// Object denoting the version of Archipelago which generated the
    /// multiworld.
    pub generator_version: NetworkVersion,

    /// Denotes special features or capabilities that the sender is capable of.
    /// Example: `WebHost`
    pub tags: Vec<String>,

    /// Denoted whether a password is required to join this room.
    #[serde(rename = "password")]
    pub password_required: bool,

    /// Mapping of Permission name to Permission, keys are: "release", "collect"
    /// and "remaining".
    pub permissions: HashMap<PermissionName, Permission>,

    /// The percentage of total locations that need to be checked to receive a
    /// hint from the server.
    pub hint_cost: i64,

    /// The amount of hint points you receive per item/location check completed.
    pub location_check_points: i64,

    /// List of games present in this multiworld.
    pub games: Vec<String>,

    /// Data versions of the individual games' data packages the server will
    /// send. Used to decide which games' caches are outdated. See Data Package
    /// Contents.
    #[deprecated(note = "Use `datapackage_checksums` instead.")]
    pub datapackage_versions: HashMap<String, i64>,

    /// Checksum hash of the individual games' data packages the server will
    /// send. Used by newer clients to decide which games' caches are outdated.
    /// See Data Package Contents for more information.
    pub datapackage_checksums: HashMap<String, String>,

    /// Uniquely identifying name of this generation
    pub seed_name: String,

    /// Unix time stamp of "now". Send for time synchronization if wanted for
    /// things like the DeathLink Bounce.
    pub time: f64,
}

/// Sent to clients when the server refuses connection. This is sent during the
/// initial connection handshake.
#[derive(Debug, Serialize, Deserialize)]
pub struct ConnectionRefused {
    /// Optional. When provided, should contain any one of: InvalidSlot,
    /// InvalidGame, IncompatibleVersion, InvalidPassword, or
    /// InvalidItemsHandling.
    pub errors: Vec<ConnectionRefusedError>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ConnectionRefusedError {
    /// InvalidSlot indicates that the sent 'name' field did not match any auth
    /// entry on the server.
    InvalidSlot,

    /// InvalidGame indicates that a correctly named slot was found, but the
    /// game for it mismatched.
    InvalidGame,

    /// IncompatibleVersion indicates a version mismatch.
    IncompatibleVersion,

    /// InvalidPassword indicates the wrong, or no password when it was
    /// required, was sent.
    InvalidPassword,

    /// InvalidItemsHandling indicates a wrong value type or flag combination
    /// was sent.
    InvalidItemsHandling,
}

/// Sent to clients when the connection handshake is successfully completed.
#[derive(Debug, Serialize, Deserialize)]
pub struct Connected {
    /// Your team number. See NetworkPlayer for more info on team number.
    pub team: i64,

    /// Your slot number on your team. See NetworkPlayer for more info on the
    /// slot number.
    pub slot: i64,

    /// List denoting other players in the multiworld, whether connected or not.
    pub players: Vec<NetworkPlayer>,

    /// Contains ids of remaining locations that need to be checked. Useful for
    /// trackers, among other things.
    pub missing_locations: Vec<i64>,

    /// Contains ids of all locations that have been checked. Useful for
    /// trackers, among other things. Location ids are in the range of ± 2^53-1.
    pub checked_locations: Vec<i64>,

    /// Contains a json object for slot related data, differs per game. Empty if
    /// not required. Not present if slot_data in Connect is false.
    pub slot_data: HashMap<String, serde_json::Value>,

    /// Maps each slot to a NetworkSlot information.
    ///
    /// TODO: the key is actually an i64, but json isn't allowed to have
    /// non-string keys, so we need to parse it.
    pub slot_info: HashMap<String, NetworkSlot>,

    /// Number of hint points that the current player has.
    pub hint_points: i64,
}

/// Sent to clients when they receive an item.
#[derive(Debug, Serialize, Deserialize)]
pub struct ReceivedItems {
    /// The next empty slot in the list of items for the receiving client.
    pub index: i64,

    /// The items which the client is receiving.
    pub items: Vec<NetworkItem>,
}

/// Sent to clients to acknowledge a received LocationScouts packet and responds
/// with the item in the location(s) being scouted.
#[derive(Debug, Serialize, Deserialize)]
pub struct LocationInfo {
    /// Contains list of item(s) in the location(s) scouted.
    pub locations: Vec<NetworkItem>,
}

/// Sent when there is a need to update information about the present game
/// session.
///
/// RoomUpdate may contain the same arguments from RoomInfo and, once
/// authenticated, arguments from Connected with the following exceptions:
///
/// - players: Sent in the event of an alias rename. Always sends all players, whether connected or not.
/// - checked_locations: May be a partial update, containing new locations that were checked, especially from a coop partner in the same slot.
/// - missing_locations: Never sent in this packet. If needed, it is the inverse of checked_locations.
///
/// All arguments for this packet are optional, only changes are sent.
#[derive(Debug, Serialize, Deserialize)]
pub struct RoomUpdate {
    // TODO: this
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum PrintJSON {
    /// A player received an item.
    ItemSend {
        data: Vec<JSONMessagePart>,
        receiving: i64,
        item: NetworkItem,
    },

    /// A player used the !getitem command.
    ItemCheat {
        data: Vec<JSONMessagePart>,
        receiving: i64,
        item: NetworkItem,
        team: i64,
    },

    /// A player hinted.
    Hint {
        data: Vec<JSONMessagePart>,
        receiving: i64,
        item: NetworkItem,
        found: bool,
    },

    /// A player connected.
    Join {
        data: Vec<JSONMessagePart>,
        team: i64,
        slot: i64,
        tags: Vec<String>,
    },

    /// A player disconnected.
    Part {
        data: Vec<JSONMessagePart>,
        team: i64,
        slot: i64,
    },

    /// A player sent a chat message.
    Chat {
        data: Vec<JSONMessagePart>,
        team: i64,
        slot: i64,
        message: String,
    },

    /// The server broadcasted a message.
    ServerChat {
        data: Vec<JSONMessagePart>,
        message: String,
    },

    /// The client has triggered a tutorial message, such as when first connecting.
    Tutorial { data: Vec<JSONMessagePart> },

    /// A player changed their tags.
    TagsChanged {
        data: Vec<JSONMessagePart>,
        team: i64,
        slot: i64,
        tags: Vec<String>,
    },

    /// Someone (usually the client) entered an ! command.
    CommandResult { data: Vec<JSONMessagePart> },

    /// The client entered an !admin command.
    AdminCommandResult { data: Vec<JSONMessagePart> },

    /// A player reached their goal.
    Goal {
        data: Vec<JSONMessagePart>,
        team: i64,
        slot: i64,
    },

    /// A player released the remaining items in their world.
    Release {
        data: Vec<JSONMessagePart>,
        team: i64,
        slot: i64,
    },

    /// A player collected the remaining items for their world.
    Collect {
        data: Vec<JSONMessagePart>,
        team: i64,
        slot: i64,
    },

    /// The current server countdown has progressed.
    Countdown {
        data: Vec<JSONMessagePart>,
        countdown: i64,
    },
}

/// Sent to clients to provide what is known as a 'data package' which contains
/// information to enable a client to most easily communicate with the
/// Archipelago server. Contents include things like location id to name
/// mappings, among others; see Data Package Contents for more info.
#[derive(Debug, Serialize, Deserialize)]
pub struct DataPackage {
    /// The data package as a JSON object.
    pub data: DataPackageObject,
}

/// Sent to clients after a client requested this message be sent to them, more
/// info in the Bounce package.
#[derive(Debug, Serialize, Deserialize)]
pub struct Bounced {
    /// Optional. Game names this message is targeting
    #[serde(default)]
    pub games: Vec<String>,

    /// Optional. Player slot IDs that this message is targeting
    #[serde(default)]
    pub slots: Vec<i64>,

    /// Optional. Client Tags this message is targeting
    #[serde(default)]
    pub tags: Vec<String>,

    /// The data in the Bounce package copied
    #[serde(default)]
    data: serde_json::Value,
}

/// Sent to clients if the server caught a problem with a packet. This only
/// occurs for errors that are explicitly checked for.
#[derive(Debug, Serialize, Deserialize)]
pub struct InvalidPacket {
    /// The PacketProblemType that was detected in the packet.
    pub r#type: PacketProblemType,

    /// The cmd argument of the faulty packet, will be None if the cmd failed to
    /// be parsed.
    pub original_cmd: Option<String>,

    /// A descriptive message of the problem at hand.
    pub text: String,
}

/// PacketProblemType indicates the type of problem that was detected in the
/// faulty packet, the known problem types are below but others may be added in
/// the future.
///
/// Other packet types may be added in the future.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum PacketProblemType {
    /// cmd argument of the faulty packet that could not be parsed correctly.
    Cmd,

    /// Arguments of the faulty packet which were not correct.
    Arguments,
}

/// Sent to clients as a response the a Get package.
#[derive(Debug, Serialize, Deserialize)]
pub struct Retrieved {
    /// A key-value collection containing all the values for the keys requested
    /// in the Get package.
    ///
    /// If a requested key was not present in the server's data, the associated
    /// value will be null.
    ///
    /// Additional arguments added to the Get package that triggered this
    /// Retrieved will also be passed along.
    pub keys: HashMap<String, serde_json::Value>,
}

/// Sent to clients in response to a Set package if want_reply was set to true, or if the client has registered to receive updates for a certain key using the SetNotify package. SetReply packages are sent even if a Set package did not alter the value for the key.
#[derive(Debug, Serialize, Deserialize)]
pub struct SetReply {
    /// The key that was updated.
    pub key: String,

    /// The new value for the key.
    pub value: serde_json::Value,

    /// The value the key had before it was updated. Not present on "_read" prefixed special keys.
    pub original_value: serde_json::Value,
}

/// Client -> Server messages
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "cmd")]
pub enum ClientMessage {
    Connect(Connect),
    Sync(SyncRequest),
    LocationChecks(LocationChecks),
    LocationScouts(LocationScouts),
    StatusUpdate(StatusUpdate),
    Say(Say),
    GetDataPackage(GetDataPackage),
    Bounce(Bounce),
    Get(Get),
    Set(Set),
    SetNotify(SetNotify),
}

/// Sent by the client to initiate a connection to an Archipelago game session.
#[derive(Debug, Serialize, Deserialize)]
pub struct Connect {
    /// If the game session requires a password, it should be passed here.
    pub password: Option<String>,

    /// The name of the game the client is playing. Example: A Link to the Past
    pub game: String,

    /// The player name for this client.
    pub name: String,

    /// Unique identifier for player client.
    pub uuid: String,

    /// An object representing the Archipelago version this client supports.
    pub version: NetworkVersion,

    /// Flags configuring which items should be sent by the server. Read below
    /// for individual flags.
    pub items_handling: ItemsHandlingFlags,

    /// Denotes special features or capabilities that the sender is capable of.
    /// Tags.
    /// TODO: switch back to pub tags: Vec<ClientTag>,
    pub tags: Vec<String>,

    /// If true, the Connect answer will contain slot_data
    pub slot_data: bool,
}

// Sent to server to request a ReceivedItems packet to synchronize items.
pub type SyncRequest = ();

#[derive(Debug, Serialize, Deserialize)]
pub struct ItemsHandlingFlags(u8);

impl ItemsHandlingFlags {
    pub const CAN_RECEIVE_ITEMS: Self = Self(0b1);
    pub const HAS_LOCAL_ITEMS: Self = Self(0b10);
    pub const REQUEST_STARTING_INVENTORY: Self = Self(0b100);

    pub fn can_receive_items(&self) -> bool {
        self.0 & Self::CAN_RECEIVE_ITEMS.0 != 0
    }

    pub fn has_local_items(&self) -> bool {
        self.can_receive_items() && self.0 & Self::HAS_LOCAL_ITEMS.0 != 0
    }

    pub fn receive_starting_inventory(&self) -> bool {
        self.can_receive_items() && self.0 & Self::REQUEST_STARTING_INVENTORY.0 != 0
    }
}

impl BitOr for ItemsHandlingFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

/// Update arguments from the Connect package, currently only updating tags and
/// items_handling is supported.
#[derive(Debug, Serialize, Deserialize)]
pub struct ConnectUpdate {
    /// Flags configuring which items should be sent by the server.
    pub items_handling: ItemsHandlingFlags,

    /// Denotes special features or capabilities that the sender is capable of.
    /// Tags.
    ///
    /// TODO: switch back to pub tags: Vec<ClientTag>,
    pub tags: Vec<String>,
}

/// Sent to server to inform it of locations that the client has checked. Used
/// to inform the server of new checks that are made, as well as to sync state.
#[derive(Debug, Serialize, Deserialize)]
pub struct LocationChecks {
    /// The ids of the locations checked by the client. May contain any number
    /// of checks, even ones sent before; duplicates do not cause issues with
    /// the Archipelago server.
    pub locations: Vec<i64>,
}

/// Sent to the server to retrieve the items that are on a specified list of
/// locations. The server will respond with a LocationInfo packet containing the
/// items located in the scouted locations. Fully remote clients without a patch
/// file may use this to "place" items onto their in-game locations, most
/// commonly to display their names or item classifications before/upon pickup.
///
/// LocationScouts can also be used to inform the server of locations the client
/// has seen, but not checked. This creates a hint as if the player had run
/// !hint_location on a location, but without deducting hint points. This is
/// useful in cases where an item appears in the game world, such as 'ledge
/// items' in A Link to the Past. To do this, set the create_as_hint parameter
/// to a non-zero value.
#[derive(Debug, Serialize, Deserialize)]
pub struct LocationScouts {
    /// The ids of the locations seen by the client. May contain any number of
    /// locations, even ones sent before; duplicates do not cause issues with
    /// the Archipelago server.
    pub locations: Vec<i64>,

    /// If non-zero, the scouted locations get created and broadcasted as a
    /// player-visible hint.
    ///
    /// If 2 only new hints are broadcast, however this does not remove them
    /// from the LocationInfo reply.
    pub create_as_hint: i64,
}

/// Sent to the server to update on the sender's status. Examples include
/// readiness or goal completion. (Example: defeated Ganon in A Link to the
/// Past)
#[derive(Debug, Serialize, Deserialize)]
pub struct StatusUpdate {
    /// One of Client States. Send as int. Follow the link for more information.
    status: ClientStatus,
}

/// Basic chat command which sends text to the server to be distributed to other
/// clients.
#[derive(Debug, Serialize, Deserialize)]
pub struct Say {
    /// Text to send to others.
    text: String,
}

/// Requests the data package from the server. Does not require client authentication.
#[derive(Debug, Serialize, Deserialize)]
pub struct GetDataPackage {
    /// Optional. If specified, will only send back the specified data. Such as,
    /// ["Factorio"] -> Datapackage with only Factorio data.
    pub games: Vec<String>,
}

/// Send this message to the server, tell it which clients should receive the
/// message and the server will forward the message to all those targets to
/// which any one requirement applies.
#[derive(Debug, Serialize, Deserialize)]
pub struct Bounce {
    /// Optional. Game names that should receive this message
    pub games: Vec<String>,

    /// Optional. Player IDs that should receive this message
    pub slots: Vec<i64>,

    /// Optional. Client tags that should receive this message
    pub tags: Vec<String>,

    /// Any data you want to send
    pub data: serde_json::Value,
}

/// Used to request a single or multiple values from the server's data storage,
/// see the Set package for how to write values to the data storage. A Get
/// package will be answered with a Retrieved package.
#[derive(Debug, Serialize, Deserialize)]
pub struct Get {
    /// Keys to retrieve the values for.
    pub keys: Vec<String>,
    // TODO: Additional arguments sent in this package will also be added to the
    // Retrieved package it triggers.
    //
    // Some special keys exist with specific return data, all of them have the
    // prefix _read_, so hints_{team}_{slot} is _read_hints_{team}_{slot}.
    //
    // - hints_{team}_{slot}    list[Hint]  All Hints belonging to the requested
    //   Player.
    // - slot_data_{slot}   dict[str, any]  slot_data belonging to the requested
    //   slot.
    // - item_name_groups_{game_name}   dict[str, list[str]]    item_name_groups
    //   belonging to the requested game.
    // - location_name_groups_{game_name}   dict[str, list[str]]
    //   location_name_groups belonging to the requested game.
    // - client_status_{team}_{slot}    ClientStatus    The current game status
    //   of the requested player.
}

/// Used to write data to the server's data storage, that data can then be
/// shared across worlds or just saved for later. Values for keys in the data
/// storage can be retrieved with a Get package, or monitored with a SetNotify
/// package. Keys that start with _read_ cannot be set.
#[derive(Debug, Serialize, Deserialize)]
pub struct Set {
    /// The key to manipulate. Can never start with "_read".
    pub key: String,

    /// The default value to use in case the key has no value on the server.
    pub default: serde_json::Value,

    /// If true, the server will send a SetReply response back to the client.
    pub want_reply: bool,

    /// Operations to apply to the value, multiple operations can be present and
    /// they will be executed in order of appearance.
    pub operations: Vec<DataStorageOperation>,
}

/// A DataStorageOperation manipulates or alters the value of a key in the data
/// storage. If the operation transforms the value from one state to another
/// then the current value of the key is used as the starting point otherwise
/// the Set's package default is used if the key does not exist on the server
/// already. DataStorageOperations consist of an object containing both the
/// operation to be applied, provided in the form of a string, as well as the
/// value to be used for that operation, Example: {"operation": "add", "value":
/// 12}
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum DataStorageOperation {
    /// Sets the current value of the key to value.
    Replace(serde_json::Value),
    /// If the key has no value yet, sets the current value of the key to
    /// default of the Set's package (value is ignored).
    Default,
    /// Adds value to the current value of the key, if both the current value
    /// and value are arrays then value will be appended to the current value.
    Add(serde_json::Value),
    /// Multiplies the current value of the key by value.
    Mul(serde_json::Value),
    /// Multiplies the current value of the key to the power of value.
    Pow(serde_json::Value),
    /// Sets the current value of the key to the remainder after division by
    /// value.
    Mod(serde_json::Value),
    /// Floors the current value (value is ignored).
    Floor,
    /// Ceils the current value (value is ignored).
    Ceil,
    /// Sets the current value of the key to value if value is bigger.
    Max(serde_json::Value),
    /// Sets the current value of the key to value if value is lower.
    Min(serde_json::Value),
    /// Applies a bitwise AND to the current value of the key with value.
    And(serde_json::Value),
    /// Applies a bitwise OR to the current value of the key with value.
    Or(serde_json::Value),
    /// Applies a bitwise Exclusive OR to the current value of the key with
    /// value.
    Xor(serde_json::Value),
    /// Applies a bitwise left-shift to the current value of the key by value.
    LeftShift(serde_json::Value),
    /// Applies a bitwise right-shift to the current value of the key by value.
    RightShift(serde_json::Value),
    /// List only: removes the first instance of value found in the list.
    Remove(serde_json::Value),
    /// List or Dict: for lists it will remove the index of the value given. for
    /// dicts it removes the element with the specified key of value.
    Pop(serde_json::Value),
    /// Dict only: Updates the dictionary with the specified elements given in
    /// value creating new keys, or updating old ones if they previously
    /// existed.
    Update(serde_json::Value),
}

/// Used to register your current session for receiving all SetReply packages of certain keys to allow your client to keep track of changes.
#[derive(Debug, Serialize, Deserialize)]
pub struct SetNotify {
    /// Keys to receive all SetReply packages for.
    pub keys: Vec<String>,
}

// Appendix types
#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkPlayer {
    pub team: i64,
    pub slot: i64,
    pub alias: String,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkItemFlags(u8);

impl NetworkItemFlags {
    pub fn is_progression(&self) -> bool {
        self.0 & 0b1 != 0
    }

    pub fn is_important(&self) -> bool {
        self.0 & 0b10 != 0
    }

    pub fn is_trap(&self) -> bool {
        self.0 & 0b100 != 0
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkItem {
    pub item: i64,
    pub location: i64,
    pub player: i64,
    pub flags: NetworkItemFlags,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum JSONMessagePart {
    PlayerId {
        text: String,
        player: i64,
    },
    PlayerName {
        text: String,
    },
    ItemId {
        text: String,
        flags: NetworkItemFlags,
        player: i64,
    },
    ItemName {
        text: String,
        flags: NetworkItemFlags,
        player: i64,
    },
    LocationId {
        text: String,
        player: i64,
    },
    LocationName {
        text: String,
        player: i64,
    },
    EntranceName {
        text: String,
    },
    Color {
        text: String,
        color: JSONColor,
    },
    #[serde(untagged)]
    Text {
        text: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JSONColor {
    Bold,
    Underline,
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    BlackBg,
    RedBg,
    GreenBg,
    YellowBg,
    BlueBg,
    MagentaBg,
    CyanBg,
    WhiteBg,
}

#[derive(Debug, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum ClientStatus {
    Unknown = 0,
    Connected = 5,
    Ready = 10,
    Playing = 20,
    Goal = 30,
}

#[derive(Debug, Deserialize)]
pub struct NetworkVersion {
    pub major: i64,
    pub minor: i64,
    pub build: i64,
}

impl serde::Serialize for NetworkVersion {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(4))?;
        map.serialize_entry("major", &self.major)?;
        map.serialize_entry("minor", &self.minor)?;
        map.serialize_entry("build", &self.build)?;
        map.serialize_entry("class", "Version")?;
        map.end()
    }
}

#[derive(Debug, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum SlotType {
    Spectator = 0,
    Player = 1,
    Group = 2,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkSlot {
    pub name: String,
    pub game: String,
    pub r#type: SlotType,
    pub group_members: Vec<i64>, // Only populated if type == Group
}

#[derive(Debug, Hash, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PermissionName {
    /// Dictates what is allowed when it comes to a player releasing their run.
    /// A release is an action which distributes the rest of the items in a
    /// player's run to those other players awaiting them.
    ///
    /// - auto: Distributes a player's items to other players when they complete
    ///   their goal.
    /// - enabled: Denotes that players may release at any time in the game.
    /// - auto-enabled: Both of the above options together.
    /// - disabled: All release modes disabled.
    /// - goal: Allows for manual use of release command once a player completes
    ///   their goal. (Disabled until goal completion)
    Release,

    /// Dictates what is allowed when it comes to a player collecting their run.
    /// A collect is an action which sends the rest of the items in a player's
    /// run.
    ///
    /// - auto: Automatically when they complete their goal.
    /// - enabled: Denotes that players may !collect at any time in the game.
    /// - auto-enabled: Both of the above options together.
    /// - disabled: All collect modes disabled.
    /// - goal: Allows for manual use of collect command once a player completes
    ///   their goal. (Disabled until goal completion)
    Collect,

    /// Dictates what is allowed when it comes to a player querying the items
    /// remaining in their run.
    ///
    /// - goal: Allows a player to query for items remaining in their run but
    ///   only after they completed their own goal.
    /// - enabled: Denotes that players may query for any items remaining in
    ///   their run (even those belonging to other players).
    /// - disabled: All remaining item query modes disabled.
    Remaining,
}

#[derive(Debug, Serialize_repr, Deserialize_repr)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum Permission {
    Disabled = 0b000,
    Enabled = 0b001,
    Goal = 0b010,
    Auto = 0b110,
    AutoEnabled = 0b111,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Hint {
    receiving_player: i64,
    finding_player: i64,
    location: i64,
    item: i64,
    found: bool,
    entrance: String,             // TODO: default to empty string
    item_flags: NetworkItemFlags, // TODO: default to 0
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DataPackageObject {
    pub games: HashMap<String, GameData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GameData {
    pub item_name_to_id: HashMap<String, i64>,
    pub location_name_to_id: HashMap<String, i64>,
    pub version: i64,
    pub checksum: String,
}

/*
#[derive(Debug, Serialize, Deserialize)]
pub enum ClientTag {
    AP,
    DeathLink,
    Tracker,
    TextOnly,
    Other(String), // TODO: ensure this serializes as expected
}

impl Into<ClientTag> for &str {
    fn into(self) -> ClientTag {
        match self {
            "AP" => ClientTag::AP,
            "DeathLink" => ClientTag::DeathLink,
            "Tracker" => ClientTag::Tracker,
            "TextOnly" => ClientTag::TextOnly,
            _ => ClientTag::Other(self.to_string()),
        }
    }
}
 */

pub struct DeathLink {
    pub time: f64,
    pub cause: Option<String>,
    pub source: String,
}
