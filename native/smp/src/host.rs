//! The SMP's view of the server: what the game's own helpers gave the old bundle (`game.native_tell`,
//! `native_open_menu`...), written once over the mod API's `ServerHost`, so the SMP's logic keeps its shape.

use mod_api::{ItemStack, PlayerRef, ServerHost, Text};
use std::path::PathBuf;

/// The server a hook was given, and the player a command acts for.
pub struct Game<'a> {
    pub h: &'a mut dyn ServerHost,
    pub acting: Option<u128>,
}

/// A player on the server, as the SMP reads them.
#[derive(Clone, Debug)]
pub struct NativeServerPlayer {
    pub uuid: u128,
    pub name: String,
    /// The operator level, 0 to 4.
    pub permission: u8,
    /// `minecraft:overworld`, `minecraft:the_nether`, `minecraft:the_end`.
    pub dimension: String,
    pub position: [f64; 3],
    /// 0 survival, 1 creative, 2 adventure, 3 spectator.
    pub game_mode: i32,
    pub health: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub entity_id: i32,
}

/// Which of a player's inventories (`/invsee`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeInventoryKind {
    Main,
    Ender,
}

impl<'a> Game<'a> {
    pub fn new(h: &'a mut dyn ServerHost) -> Self {
        Self { h, acting: None }
    }

    /// The open world's folder.
    pub fn native_world_dir(&self) -> Option<PathBuf> {
        self.h.world_dir()
    }

    pub fn player_ref(&self, uuid: u128) -> Option<PlayerRef> {
        self.h.player(uuid)
    }

    pub fn native_server_players(&self) -> Vec<NativeServerPlayer> {
        let h: &dyn ServerHost = &*self.h;
        h.players()
            .server_players()
            .into_iter()
            .filter_map(|p| read_player(h, p))
            .collect()
    }

    /// A player on the server, as the SMP reads them.
    pub fn player_info(&self, uuid: u128) -> Option<NativeServerPlayer> {
        let h: &dyn ServerHost = &*self.h;
        read_player(h, h.player(uuid)?)
    }

    pub fn native_acting_player(&self) -> Option<NativeServerPlayer> {
        let h: &dyn ServerHost = &*self.h;
        let p = h.players().acting_player()?;
        read_player(h, p)
    }

    pub fn native_tell(&mut self, uuid: u128, line: Text) -> bool {
        let Some(p) = self.h.player(uuid) else {
            return false;
        };
        p.tell(self.h, line).unwrap_or(false)
    }

    pub fn native_action_bar(&mut self, uuid: u128, line: Text) -> bool {
        let Some(p) = self.h.player(uuid) else {
            return false;
        };
        p.action_bar(self.h, line).unwrap_or(false)
    }

    pub fn native_tab_list(&mut self, uuid: u128, header: &Text, footer: &Text) -> bool {
        let Some(p) = self.h.player(uuid) else {
            return false;
        };
        let board = {
            let mut tabs = TABS.lock().unwrap_or_else(|e| e.into_inner());
            match tabs.iter_mut().find(|t| t.0 == uuid) {
                Some(t) => {
                    t.2 = footer.clone();
                    t.1.clone()
                }
                None => {
                    tabs.push((uuid, blank(), footer.clone()));
                    blank()
                }
            }
        };
        // The player's board over the server's header.
        let header = if board == blank() {
            header.clone()
        } else {
            board.append(Text::literal("\n")).append(header.clone())
        };
        p.tab_list(self.h, &header, footer).unwrap_or(false)
    }

    pub fn native_broadcast(&mut self, line: Text) {
        let _ = self.h.messages().broadcast(line);
    }

    /// Feedback to whoever ran the command.
    pub fn native_feedback(&mut self, line: Text) {
        if let Some(uuid) = self.acting {
            self.native_tell(uuid, line);
        } else {
            let _ = self.h.commands().feedback(line);
        }
    }

    pub fn native_run_command(&mut self, line: &str) -> bool {
        self.h.commands().run_command(line)
    }

    pub fn native_text_allowed(&self, text: &str) -> Option<bool> {
        self.h.messages().text_allowed(text)
    }

    pub fn native_tick_stats(&self) -> (f32, f64) {
        self.h.tick_stats()
    }

    pub fn native_set_nickname(&mut self, uuid: u128, nickname: Option<&str>) -> bool {
        let Some(p) = self.h.player(uuid) else {
            return false;
        };
        p.set_nickname(self.h, nickname)
    }

    pub fn native_hide_player(&mut self, uuid: u128, level: Option<u8>) -> bool {
        let Some(p) = self.h.player(uuid) else {
            return false;
        };
        p.hide(self.h, level)
    }

    pub fn native_sleep_ignored(&mut self, uuid: u128, ignored: bool) -> bool {
        let Some(p) = self.h.player(uuid) else {
            return false;
        };
        p.set_sleep_ignored(self.h, ignored)
    }

    pub fn native_send_map(&mut self, uuid: u128, map_id: i32, colors: &[u8]) -> bool {
        let Some(p) = self.h.player(uuid) else {
            return false;
        };
        p.send_map(self.h, map_id, colors)
    }

    /// Opens a chest menu of `rows` rows over `items`: its window id.
    pub fn native_open_menu(
        &mut self,
        uuid: u128,
        rows: u8,
        title: Text,
        items: Vec<Option<ItemStack>>,
    ) -> Option<i32> {
        self.h
            .open_menu(uuid, rows, title, items, mod_api::MenuReplace::CloseFirst)
            .ok()
            .flatten()
    }

    /// Replaces the items of the menu the player has open.
    pub fn native_set_menu_items(&mut self, uuid: u128, items: Vec<Option<ItemStack>>) {
        let Some(p) = self.h.player(uuid) else { return };
        let Some(window) = p
            .container_menu(&*self.h)
            .and_then(|m| m.container_id(&*self.h))
        else {
            return;
        };
        let _ = self.h.menu_set_items(uuid, window, items);
    }

    pub fn native_close_menu(&mut self, uuid: u128) {
        self.h.close_menu(uuid);
    }

    /// The item id of `stack` (`minecraft:stone`).
    pub fn item_id(&self, stack: &ItemStack) -> String {
        stack.item().id(&*self.h)
    }

    pub fn max_stack_size(&self, item: &str) -> Option<u32> {
        let key = self.h.item_key(&mod_api::Identifier::parse(item).ok()?)?;
        self.h.max_stack_size(key)
    }

    /// A stack of `count` `item` (an id), None for an unknown item.
    pub fn stack_of(&self, item: &str, count: u32) -> Option<ItemStack> {
        let key = self.h.item_key(&mod_api::Identifier::parse(item).ok()?)?;
        ItemStack::new(key, count)
    }
}

fn read_player(h: &dyn ServerHost, p: PlayerRef) -> Option<NativeServerPlayer> {
    let entity = p.entity();
    let [yaw, pitch] = entity.rotation(h).unwrap_or([0.0; 2]);
    Some(NativeServerPlayer {
        uuid: p.uuid(),
        name: p.name(h)?,
        permission: p.permission_level(h).unwrap_or(0),
        dimension: p.dimension(h).map(|d| d.to_string()).unwrap_or_default(),
        position: entity.position(h).unwrap_or_default(),
        game_mode: p.game_mode(h).map_or(0, |m| m as i32),
        health: 20.0,
        yaw,
        pitch,
        entity_id: 0,
    })
}

// ---- items and inventories ----

/// The first slot past the main inventory: 36 to 39 are armour, 40 the off hand.
pub const MAIN: usize = 36;
pub const OFFHAND: usize = 40;

/// Whether `s` has no damage and no component changes: what the market and a trade take as "plain".
pub fn plain(s: &ItemStack) -> bool {
    s.damage_component().is_none() && s.components().is_none_or(|c| c.as_nbt().is_empty())
}

/// The inventory slot (0..9 the hotbar, 9..36 the rest) a click on `slot` of a chest menu of `size` slots
/// landed on, when it is the player's inventory below the menu.
pub fn native_menu_inventory_slot(size: usize, slot: i32) -> Option<usize> {
    let n = usize::try_from(slot).ok()?.checked_sub(size)?;
    match n {
        0..27 => Some(n + 9),
        27..36 => Some(n - 27),
        _ => None,
    }
}

/// Adds `stack` to `slots[range]` as `Inventory.add` places it (onto equal stacks, then into empty slots), up to
/// `max` a slot; what did not fit stays in `stack`. True when it all went in.
pub fn add_to(
    slots: &mut [Option<ItemStack>],
    range: std::ops::Range<usize>,
    stack: &mut ItemStack,
    max: u32,
) -> bool {
    let max = max.max(1);
    for i in range.clone() {
        if stack.count() == 0 {
            break;
        }
        if let Some(s) = slots[i].as_mut()
            && s.is_stackable_with(stack)
            && s.count() < max
        {
            let n = (max - s.count()).min(stack.count());
            let _ = s.grow(n);
            moved(stack, n);
        }
    }
    for i in range {
        if stack.count() == 0 {
            break;
        }
        if slots[i].is_none() {
            let n = max.min(stack.count());
            let mut put = stack.clone();
            let _ = put.set_count(n);
            slots[i] = Some(put);
            moved(stack, n);
        }
    }
    stack.count() == 0
}

/// `stack` less `n`: a stack at zero stays at count 1 and `is_empty` says it is spent.
fn moved(stack: &mut ItemStack, n: u32) {
    let left = stack.count() - n;
    if left > 0 {
        let _ = stack.set_count(left);
    } else {
        *stack = stack.clone();
        SPENT.with(|s| s.set(true));
    }
}

thread_local! {
    static SPENT: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

impl<'a> Game<'a> {
    fn pref(&self, uuid: u128) -> Option<PlayerRef> {
        self.h.player(uuid)
    }

    /// A player's inventory slots 0..=40, empty ones None; None for a player who isn't on.
    pub fn native_player_stacks(&self, uuid: u128) -> Option<Vec<Option<ItemStack>>> {
        let p = self.pref(uuid)?;
        let mut out = vec![None; OFFHAND + 1];
        for (slot, stack) in p.inventory_slots(&*self.h)? {
            if let Some(cell) = out.get_mut(slot as usize) {
                *cell = Some(stack);
            }
        }
        Some(out)
    }

    pub fn native_max_stack_size(&self, item: &str) -> Option<u32> {
        self.max_stack_size(item)
    }

    pub fn set_slot(
        &mut self,
        uuid: u128,
        slot: usize,
        stack: Option<ItemStack>,
    ) -> Result<(), String> {
        let p = self.pref(uuid).ok_or("the player is not on the server")?;
        p.set_inventory_slot(self.h, slot as u32, stack)
            .map_err(|e| format!("{e:?}"))
    }

    /// The acting player's main hand: its item, count, and whether it is plain.
    pub fn native_held(&self) -> Option<(String, u32, bool)> {
        let me = self.acting?;
        let p = self.pref(me)?;
        let slot = p.selected_slot(&*self.h)?;
        let stacks = self.native_player_stacks(me)?;
        let s = stacks.get(slot)?.as_ref()?;
        Some((self.item_id(s), s.count(), plain(s)))
    }

    fn acting_stacks(&self) -> Vec<Option<ItemStack>> {
        let mut v = self
            .acting
            .and_then(|u| self.native_player_stacks(u))
            .unwrap_or_default();
        v.retain(|_| true);
        v
    }

    /// Every plain item the acting player carries (36 slots and the off hand), by id.
    pub fn native_carried_items(&self) -> Vec<(String, u32)> {
        let stacks = self.acting_stacks();
        let mut out: Vec<(String, u32)> = Vec::new();
        for s in stacks
            .iter()
            .take(MAIN)
            .chain(stacks.get(OFFHAND))
            .flatten()
            .filter(|s| plain(s))
        {
            let id = self.item_id(s);
            match out.iter_mut().find(|(i, _)| *i == id) {
                Some((_, n)) => *n += s.count(),
                None => out.push((id, s.count())),
            }
        }
        out
    }

    fn carried_slots(&self) -> Vec<usize> {
        (0..MAIN).chain([OFFHAND]).collect()
    }

    /// How many of plain `item` the acting player carries.
    pub fn native_count_items(&mut self, item: &str) -> u32 {
        let stacks = self.acting_stacks();
        self.carried_slots()
            .into_iter()
            .filter_map(|i| stacks.get(i)?.as_ref())
            .filter(|s| plain(s) && self.item_id(s) == item)
            .map(|s| s.count())
            .sum()
    }

    /// Takes up to `count` plain `item`, the main hand first; what it took.
    pub fn native_take_items(&mut self, item: &str, count: u32) -> u32 {
        let Some(me) = self.acting else { return 0 };
        let Some(p) = self.pref(me) else { return 0 };
        let selected = p.selected_slot(&*self.h).unwrap_or(0);
        let mut stacks = self.acting_stacks();
        let mut order = vec![selected];
        order.extend(self.carried_slots().into_iter().filter(|&i| i != selected));
        let mut left = count;
        for i in order {
            if left == 0 {
                break;
            }
            let Some(Some(s)) = stacks.get(i) else {
                continue;
            };
            if !plain(s) || self.item_id(s) != item {
                continue;
            }
            let n = s.count().min(left);
            let rest = (s.count() > n).then(|| {
                let mut r = s.clone();
                let _ = r.set_count(s.count() - n);
                r
            });
            if self.set_slot(me, i, rest.clone()).is_ok() {
                left -= n;
                stacks[i] = rest;
            }
        }
        count - left
    }

    /// How many plain `item` still fit; 0 for an unknown item.
    pub fn native_room_for(&mut self, item: &str) -> u32 {
        let Some(max) = self.max_stack_size(item) else {
            return 0;
        };
        let stacks = self.acting_stacks();
        self.carried_slots()
            .into_iter()
            .filter(|&i| i < MAIN)
            .map(|i| match stacks.get(i) {
                Some(Some(s)) if plain(s) && self.item_id(s) == item => {
                    max.saturating_sub(s.count())
                }
                Some(None) | None => max,
                Some(Some(_)) => 0,
            })
            .sum()
    }

    /// Gives up to `count` plain `item` where they fit; what it gave (nothing is dropped).
    pub fn native_give_items(&mut self, item: &str, count: u32) -> u32 {
        let Some(me) = self.acting else { return 0 };
        let give = count.min(self.native_room_for(item));
        let Some(max) = self.max_stack_size(item) else {
            return 0;
        };
        let mut left = give;
        while left > 0 {
            let n = left.min(max.max(1));
            let Some(stack) = self.stack_of(item, n) else {
                break;
            };
            let Some(p) = self.pref(me) else { break };
            if p.give_stack(self.h, stack).is_err() {
                break;
            }
            left -= n;
        }
        give - left
    }

    /// Gives `stack` whole to the player `uuid`, as `Inventory.add` places it (the 36 main slots, never the
    /// armour): what did not fit comes back (nothing is dropped), all of it for a player who isn't here.
    pub fn native_give_stack(&mut self, uuid: u128, stack: ItemStack) -> Option<ItemStack> {
        let Some(p) = self.pref(uuid) else {
            return Some(stack);
        };
        let max = self.h.max_stack_size(stack.item()).unwrap_or(64).max(1);
        let Some(mut slots) = self.native_player_stacks(uuid) else {
            return Some(stack);
        };
        let mut rest = stack.clone();
        SPENT.with(|s| s.set(false));
        let all = add_to(&mut slots, 0..MAIN, &mut rest, max);
        let fits = if all {
            stack.count()
        } else {
            stack.count() - rest.count()
        };
        if all || SPENT.with(|s| s.get()) {
            SPENT.with(|s| s.set(false));
        }
        let mut give = fits;
        while give > 0 {
            let n = give.min(max);
            let mut part = stack.clone();
            if part.set_count(n).is_err() || p.give_stack(self.h, part).is_err() {
                break;
            }
            give -= n;
        }
        let given = fits - give;
        let left = stack.count() - given;
        if left == 0 {
            None
        } else {
            let mut r = stack;
            let _ = r.set_count(left);
            Some(r)
        }
    }

    /// The two offers swap in one step: both are checked first, so nothing moves unless all of it can.
    pub fn native_exchange_items(
        &mut self,
        a: u128,
        a_offer: &[(usize, ItemStack)],
        b: u128,
        b_offer: &[(usize, ItemStack)],
    ) -> Result<(), String> {
        let mut sa = self
            .native_player_stacks(a)
            .ok_or("a player is not on the server")?;
        let mut sb = self
            .native_player_stacks(b)
            .ok_or("a player is not on the server")?;
        for (slots, offer) in [(&mut sa, a_offer), (&mut sb, b_offer)] {
            for (i, want) in offer {
                if slots.get(*i).and_then(Option::as_ref) != Some(want) {
                    return Err("a stack changed".into());
                }
                slots[*i] = None;
            }
        }
        for (slots, gets) in [(&mut sa, b_offer), (&mut sb, a_offer)] {
            for (_, stack) in gets {
                let max = self.h.max_stack_size(stack.item()).unwrap_or(64);
                let mut rest = stack.clone();
                SPENT.with(|s| s.set(false));
                add_to(slots, 0..MAIN, &mut rest, max);
                if !SPENT.with(|s| s.replace(false)) {
                    return Err("there is no room".into());
                }
            }
        }
        for (uuid, offer) in [(a, a_offer), (b, b_offer)] {
            for (i, _) in offer {
                self.set_slot(uuid, *i, None)?;
            }
        }
        for (uuid, gets) in [(a, b_offer), (b, a_offer)] {
            for (_, stack) in gets {
                if let Some(rest) = self.native_give_stack(uuid, stack.clone()) {
                    // The room was checked: what is left is dropped at their feet by the game's own give.
                    if let Some(p) = self.pref(uuid) {
                        let _ = p.give_stack(self.h, rest);
                    }
                }
            }
        }
        Ok(())
    }

    /// A player's inventory (`Player`: 41 slots, 36 to 39 the armour, 40 the off hand; `Ender`: 27) and whether
    /// they are on the server. Only players who are on can be read.
    pub fn native_inventory(
        &self,
        uuid: u128,
        kind: NativeInventoryKind,
    ) -> Result<NativeInventory, String> {
        let p = self.pref(uuid).ok_or("the player is not on the server")?;
        let (len, held) = match kind {
            NativeInventoryKind::Main => (OFFHAND + 1, p.inventory_slots(&*self.h)),
            NativeInventoryKind::Ender => (27, p.ender_chest(&*self.h)),
        };
        let held = held.ok_or("the player's items can't be read")?;
        let mut slots = vec![None; len];
        for (slot, stack) in held {
            if let Some(cell) = slots.get_mut(slot as usize) {
                *cell = Some(stack);
            }
        }
        Ok(NativeInventory {
            slots,
            online: true,
        })
    }

    /// Moves the stack in `from` (which must still be `expected`) into `to`'s inventory.
    pub fn native_move_stack(
        &mut self,
        from: (u128, NativeInventoryKind, usize),
        expected: &ItemStack,
        to: (u128, NativeInventoryKind),
    ) -> Result<(), String> {
        if (from.0, from.1) == to {
            return Err("the same inventory".into());
        }
        let source = self.native_inventory(from.0, from.1)?;
        if source.slots.get(from.2).and_then(Option::as_ref) != Some(expected) {
            return Err("that stack changed".into());
        }
        let mut target = self.native_inventory(to.0, to.1)?.slots;
        let max = self.h.max_stack_size(expected.item()).unwrap_or(64);
        let range = match to.1 {
            NativeInventoryKind::Main => 0..MAIN,
            NativeInventoryKind::Ender => 0..27,
        };
        let before = target.clone();
        let mut rest = expected.clone();
        SPENT.with(|s| s.set(false));
        add_to(&mut target, range, &mut rest, max);
        if !SPENT.with(|s| s.replace(false)) {
            return Err("no room".into());
        }
        let (Some(p_to), Some(p_from)) = (self.pref(to.0), self.pref(from.0)) else {
            return Err("the player is not on the server".into());
        };
        for (i, (new, old)) in target.iter().zip(&before).enumerate() {
            if new != old {
                match to.1 {
                    NativeInventoryKind::Main => {
                        p_to.set_inventory_slot(self.h, i as u32, new.clone())
                    }
                    NativeInventoryKind::Ender => {
                        p_to.set_ender_chest_slot(self.h, i as u32, new.clone())
                    }
                }
                .map_err(|e| format!("{e:?}"))?;
            }
        }
        match from.1 {
            NativeInventoryKind::Main => p_from.set_inventory_slot(self.h, from.2 as u32, None),
            NativeInventoryKind::Ender => p_from.set_ender_chest_slot(self.h, from.2 as u32, None),
        }
        .map_err(|e| format!("{e:?}"))?;
        Ok(())
    }
}

/// A player's inventory as `native_inventory` reads it.
#[derive(Clone, Debug, PartialEq)]
pub struct NativeInventory {
    pub slots: Vec<Option<ItemStack>>,
    pub online: bool,
}

// ---- menu items ----

use mod_api::{Compound, Tag};

/// A text component as NBT (`{text, italic: false, color}`: a name or lore line that isn't in italics).
fn component(text: &str, color: &str) -> Tag {
    let mut c = Compound::new();
    c.insert("text".into(), Tag::String(text.into()));
    c.insert("italic".into(), Tag::Byte(0));
    c.insert("color".into(), Tag::String(color.into()));
    Tag::Compound(c)
}

impl Game<'_> {
    /// `count` of `item` named `name` (not in italics, as a name the game gives); a plain stone for an unknown item.
    pub fn named_stack(&self, item: &str, count: u32, name: &str, color: &str) -> ItemStack {
        let mut stack = self
            .stack_of(item, count.max(1))
            .or_else(|| self.stack_of("minecraft:stone", count.max(1)))
            .expect("the stone item exists");
        let mut patch = Compound::new();
        patch.insert("minecraft:custom_name".into(), component(name, color));
        let _ = stack.apply_components(&patch);
        stack
    }

    pub fn button(&self, item: &str, name: &str, color: &str) -> ItemStack {
        self.named_stack(item, 1, name, color)
    }

    /// Lines under an item's name in a menu.
    pub fn with_lore(&self, mut stack: ItemStack, lines: &[(String, &str)]) -> ItemStack {
        let lines = lines
            .iter()
            .map(|(text, color)| component(text, color))
            .collect();
        let mut patch = Compound::new();
        patch.insert("minecraft:lore".into(), Tag::List(lines));
        let _ = stack.apply_components(&patch);
        stack
    }

    pub fn with_map_id(&self, mut stack: ItemStack, id: i32) -> ItemStack {
        let mut patch = Compound::new();
        patch.insert("minecraft:map_id".into(), Tag::Int(id));
        let _ = stack.apply_components(&patch);
        stack
    }

    /// The map id of a filled map, if it has one.
    pub fn map_id(&self, stack: &ItemStack) -> Option<i32> {
        match stack
            .components()?
            .as_nbt()
            .get(&"minecraft:map_id".into())?
        {
            Tag::Int(id) => Some(*id),
            _ => None,
        }
    }
}

// ---- sidebar, tab list, chunk tickets ----

/// The sidebar the game's helpers once drew: a title and lines of a label and a value.
pub struct NativeSidebar {
    pub title: Text,
    pub lines: Vec<(Text, Option<Text>)>,
}

static TABS: std::sync::Mutex<Vec<(u128, Text, Text)>> = std::sync::Mutex::new(Vec::new());

fn blank() -> Text {
    Text::literal("")
}

/// The ticket controller `/rtp`'s chunks are held under, registered at startup.
pub static TICKETS: std::sync::OnceLock<mod_api::TicketController> = std::sync::OnceLock::new();

impl Game<'_> {
    /// Whether the SMP runs in this world (it does wherever it is installed).
    pub fn native_on(&self, _mod_id: &str) -> bool {
        true
    }

    /// The Tab list as a player's board: the game has no per-player sidebar to write to, so the board's title and
    /// lines are the Tab list's header, over the server's footer.
    pub fn native_sidebar(&mut self, uuid: u128, sidebar: Option<&NativeSidebar>) -> bool {
        let header = match sidebar {
            None => blank(),
            Some(b) => {
                let mut t = b.title.clone();
                for (label, value) in &b.lines {
                    t = t.append(Text::literal("\n")).append(label.clone());
                    if let Some(v) = value {
                        t = t.append(Text::literal(": ")).append(v.clone());
                    }
                }
                t
            }
        };
        let footer = {
            let mut tabs = TABS.lock().unwrap_or_else(|e| e.into_inner());
            match tabs.iter_mut().find(|t| t.0 == uuid) {
                Some(t) => {
                    t.1 = header.clone();
                    t.2.clone()
                }
                None => {
                    tabs.push((uuid, header.clone(), blank()));
                    blank()
                }
            }
        };
        let Some(p) = self.h.player(uuid) else {
            return false;
        };
        p.tab_list(self.h, &header, &footer).unwrap_or(false)
    }

    /// Holds (or lets go) a chunk loaded for `owner` while `/rtp` looks at it.
    pub fn native_force_chunk_in(
        &mut self,
        _owner: &str,
        dimension: &str,
        chunk: [i32; 2],
        hold: bool,
    ) -> bool {
        let Some(controller) = TICKETS.get().copied() else {
            return false;
        };
        let Some(level) = self.h.level(dimension) else {
            return false;
        };
        let Some(chunk) = level.chunk_ref(&*self.h, chunk) else {
            return false;
        };
        chunk.set_mod_forced(
            self.h,
            controller,
            mod_api::TicketOwner::Entity(RTP_OWNER),
            hold,
            false,
        )
    }
}

/// The ticket owner of `/rtp`'s chunks.
const RTP_OWNER: u128 = 0x736d_705f_7274_7000_0000_0000_0000_0001;
