use rand::{
    rngs::SmallRng,
    SeedableRng,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    hash::Hash,
};

#[cfg(not(debug_assertions))]
pub const SPEED: u64 = 2;
#[cfg(debug_assertions)]
pub const SPEED: u64 = 1;
pub const ONE_MINUTE: u64 = 60;
pub const ONE_HOUR: u64 = ONE_MINUTE * 60;
pub const ONE_DAY: u64 = ONE_HOUR * 24;

pub type UserId = i64;
pub type Time = u64;
pub type Seed = u64;


#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Event {
    Tick,
    AddPlayer(UserId, String),
    EditPlayer(UserId, String),
    RemovePlayer(UserId),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EventData {
    pub event: Event,
    pub user_id: Option<UserId>,
    pub seed: Seed,
    pub event_idx: EventIndex,
}

impl EventData {
    pub fn filter(&self, _receiver: UserId) -> bool {
        /*
        let EventData { event, user_id } = self;
        let user_id = *user_id;

        match event {
            _ => true,
        }
        */

        true
    }
}

pub type EventIndex = u64;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Req {
    Event(Event),
}

#[derive(Serialize, Deserialize, Clone)]
pub enum Res {
    Sync(SyncData),
    Event(EventData),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SyncData {
    pub user_id: UserId,
    pub state: State,
}

// MODIFY EVENTS AND STATE BELOW



#[derive(Serialize, Deserialize, Default, Clone, Debug, Hash)]
pub struct State {
    pub players: BTreeMap<UserId, Player>,
    pub time: Time,
    pub next_event_idx: EventIndex,
}

impl State {
    pub fn update(
        &mut self,
        EventData {
            event,
            seed,
            user_id: _,
            event_idx,
        }: EventData,
    ) -> Option<()> {
        if event_idx < self.next_event_idx {
            return Some(());
        } else if event_idx > self.next_event_idx {
            return None;
        } else {
            self.next_event_idx += 1;
        }


        
        match event {
            Event::Tick => {
                let mut _rng: SmallRng = SmallRng::seed_from_u64(seed);

                self.time += 1;
            }
            Event::AddPlayer(user_id, username) => {
                let player = Player::new(username, self.time);
                self.players.insert(user_id, player);
            }
            Event::EditPlayer(user_id, username) => {
                self.players.get_mut(&user_id)?.username = username;
            }
            Event::RemovePlayer(user_id) => {
                self.players.remove(&user_id);
            }
        }

        Some(())
    }

    pub fn view(&self, _receiver: UserId) -> Self {
        State { ..self.clone() }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Hash)]
pub struct Player {
    pub username: String,
    last_online: Time,
}

impl Player {
    pub fn new(username: String, time: Time) -> Self {
        Player {
            username,
            last_online: time,
        }
    }

    pub fn is_online(&self, time: Time) -> bool {
        (time - self.last_online) / SPEED < ONE_MINUTE * 3
    }

    pub fn is_active(&self, time: Time) -> bool {
        (time - self.last_online) / SPEED < ONE_DAY
    }
}

