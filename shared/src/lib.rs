use rand::{rngs::SmallRng, seq::SliceRandom, SeedableRng};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
};
use strum::{Display, EnumCount, EnumIter, IntoEnumIterator};

#[cfg(not(debug_assertions))]
pub const SPEED: u64 = 2;
#[cfg(debug_assertions)]
pub const SPEED: u64 = 1;
pub const ONE_MINUTE: u64 = 60;
pub const ONE_HOUR: u64 = ONE_MINUTE * 60;
pub const ONE_DAY: u64 = ONE_HOUR * 24;

pub const QTY_LEAST: u64 = 6;
pub const QTY_GAP: u64 = 2;
pub const QTY_MOST: u64 = QTY_LEAST + (Veggie::COUNT as u64 - 1) * QTY_GAP;
pub const QTY_TOTAL: u64 = (QTY_LEAST + QTY_MOST) / 2 * Veggie::COUNT as u64;

pub type UserId = i64;
pub type Time = u64;
pub type Seed = u64;
pub type Quantity = u64;
pub type Money = u64;
pub type Success = bool;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Event {
    Tick,
    AddPlayer(UserId, String),
    EditPlayer(UserId, String),
    RemovePlayer(UserId),
    Trade(usize, UserId, usize),
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

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct State {
    pub players: HashMap<UserId, Player>,
    pub time: Time,
    pub next_event_idx: EventIndex,
}

impl State {
    pub fn update(
        &mut self,
        EventData {
            event,
            seed,
            user_id,
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

        let mut rng: SmallRng = SmallRng::seed_from_u64(seed);

        match event {
            Event::Tick => {
                self.time += 1;
            }
            Event::AddPlayer(user_id, username) => {
                let player = Player::new(username, self.time, &mut rng);
                self.players.insert(user_id, player);
            }
            Event::EditPlayer(user_id, username) => {
                self.players.get_mut(&user_id)?.username = username;
            }
            Event::RemovePlayer(user_id) => {
                self.players.remove(&user_id);
            }
            Event::Trade(visitor_truck, visited, visited_truck) => {
                if let Some(visitor) = user_id {
                    self.trade(visitor, visited, visitor_truck, visited_truck);
                }
            }
        }

        Some(())
    }

    pub fn view(&self, _receiver: UserId) -> Self {
        State { ..self.clone() }
    }

    pub fn trade(
        &mut self,
        visitor: UserId,
        visited: UserId,
        visitor_truck: usize,
        visited_truck: usize,
    ) {
        let visitor_unloaded_veggies = self
            .players
            .get_mut(&visitor)
            .and_then(|p| p.farm.trucks.get_mut(visitor_truck))
            .and_then(|t| t.veggies.take());
        let visited_unloaded_veggies = self
            .players
            .get_mut(&visited)
            .and_then(|p| p.farm.trucks.get_mut(visited_truck))
            .and_then(|t| t.veggies.take());

        let (visitor_veggies_to_load, visited_veggies_to_load) =
            match (visitor_unloaded_veggies, visited_unloaded_veggies) {
                (Some(mut visitor_unloaded_veggies), Some(visited_unloaded_veggies)) => {
                    // Try to plant veggies here.
                    if let Some(visited_farm) = self.players.get_mut(&visited).map(|p| &mut p.farm)
                    {
                        visited_farm.plant_veggies(&mut visitor_unloaded_veggies);
                    }

                    if visitor_unloaded_veggies.is_empty() {
                        // Trade successful
                        (Some(visited_unloaded_veggies), None)
                    } else {
                        // Trade unsuccessful
                        (
                            Some(visitor_unloaded_veggies),
                            Some(visited_unloaded_veggies),
                        )
                    }
                }
                // Invalid indices.
                (visitor_unloaded_veggies, visited_unloaded_veggies) => {
                    (visitor_unloaded_veggies, visited_unloaded_veggies)
                }
            };

        if let Some(visitor_truck) = self
            .players
            .get_mut(&visitor)
            .and_then(|p| p.farm.trucks.get_mut(visitor_truck))
        {
            visitor_truck.veggies = visitor_veggies_to_load;
        }
        if let Some(visited_truck) = self
            .players
            .get_mut(&visited)
            .and_then(|p| p.farm.trucks.get_mut(visited_truck))
        {
            visited_truck.veggies = visited_veggies_to_load;
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Player {
    pub username: String,
    last_online: Time,
    pub farm: Farm,
}

impl Player {
    pub fn new(username: String, time: Time, rng: &mut SmallRng) -> Self {
        Player {
            username,
            last_online: time,
            farm: Farm::new(rng),
        }
    }

    pub fn is_online(&self, time: Time) -> bool {
        (time - self.last_online) / SPEED < ONE_MINUTE * 3
    }

    pub fn is_active(&self, time: Time) -> bool {
        (time - self.last_online) / SPEED < ONE_DAY
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Farm {
    pub fields: Vec<Field>,
    pub trucks: Vec<Truck>,
    pub silos: Vec<Silo>,
    pub tractors: Vec<Tractor>,
    pub money: Money,
}

impl Farm {
    pub fn new(rng: &mut SmallRng) -> Self {
        Farm {
            money: 0,
            fields: Vec::new(),
            silos: vec![Silo::new(rng)],
            trucks: Vec::new(),
            tractors: vec![Tractor::new()],
        }
    }

    pub fn tick(&mut self) {}

    pub fn plant_field(&mut self, silo_index: usize) {
        if let Some(mut veggies) = self
            .silos
            .get_mut(silo_index)
            .and_then(|s| s.storage.pop_front())
        {
            self.plant_veggies(&mut veggies);

            if !veggies.is_empty() {
                if let Some(silo) = self.silos.get_mut(silo_index) {
                    silo.storage.push_front(veggies);
                }
            }
        }
    }

    pub fn plant_veggies(&mut self, veggies: &mut VeggieQty) {
        for field in &mut self.fields {
            field.plant(veggies);
        }
    }

    pub fn harvest_field(&mut self, field_index: usize) {
        if let Some(field) = self.fields.get_mut(field_index) {
            self.money += field.harvest()
        }
    }

    pub fn load_truck(&mut self, silo_index: usize) {
        if let Some(mut veggies) = self
            .silos
            .get_mut(silo_index)
            .and_then(|s| s.storage.pop_front())
        {
            for truck in &mut self.trucks {
                truck.load(&mut veggies);
            }

            if !veggies.is_empty() {
                if let Some(silo) = self.silos.get_mut(silo_index) {
                    silo.storage.push_front(veggies);
                }
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Tractor {
    pub wait: Time,
}

impl Tractor {
    pub fn new() -> Self {
        Tractor { wait: 0 }
    }

    pub fn tick(&mut self) {
        if self.wait > 0 {
            self.wait -= 1;
        }
    }
    pub fn is_ready(&self) -> bool {
        self.wait == 0
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Truck {
    veggies: Option<VeggieQty>,
    pub wait: Time,
}

impl Truck {
    pub fn new() -> Self {
        Truck {
            veggies: None,
            wait: 0,
        }
    }

    pub fn load(&mut self, to_plant: &mut VeggieQty) {
        if let Some(curr) = &mut self.veggies {
            curr.add(to_plant);
        } else {
            self.veggies = Some(to_plant.take().with_max(1));
        }
    }

    pub fn tick(&mut self) {
        if self.wait > 0 {
            self.wait -= 1;
        }
    }

    pub fn is_ready(&self) -> bool {
        self.wait == 0
    }
}

#[derive(
    Serialize, Deserialize, Clone, Copy, Debug, Display, EnumCount, EnumIter, Eq, PartialEq,
)]
#[strum(serialize_all = "title_case")]
pub enum Veggie {
    Carrot,
    Potato,
}

impl Veggie {
    pub fn qty(self) -> Quantity {
        QTY_LEAST + (self as u64) * QTY_GAP
    }

    pub fn value(self, qty: Quantity) -> Money {
        (QTY_TOTAL / self.qty()).pow(qty as u32)
    }

    pub fn image_name(self) -> PathBuf {
        PathBuf::from(self.to_string().replace(" ", "_").to_lowercase()).with_extension("png")
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Eq, PartialEq)]
pub struct VeggieQty {
    veggie: Veggie,
    qty: Quantity,
    max: Option<Quantity>,
}

impl VeggieQty {
    pub fn new(veggie: Veggie, qty: Quantity) -> Self {
        VeggieQty {
            veggie,
            qty,
            max: None,
        }
    }

    pub fn with_max(mut self, max: Quantity) -> Self {
        self.max = Some(max);
        self
    }

    pub fn qty(&self) -> Quantity {
        self.qty
    }

    pub fn veggie(&self) -> Veggie {
        self.veggie
    }

    pub fn add(&mut self, other: &mut Self) {
        if self.veggie == other.veggie {
            if let Some(max) = self.max {
                let remaining = max - self.qty;
                let to_add = other.qty.min(remaining);
                self.qty += to_add;
                other.qty -= to_add;
            }
        }
    }

    pub fn take(&mut self) -> Self {
        let mut new = VeggieQty::new(self.veggie, self.qty);
        new.add(self);
        new
    }

    pub fn value(&self) -> Money {
        self.veggie.value(self.qty)
    }

    pub fn is_empty(&self) -> bool {
        self.qty == 0
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Field {
    pub veggies: Option<VeggieQty>,
    pub max_veggies: Quantity,
}

impl Field {
    pub fn new() -> Self {
        Field {
            veggies: None,
            max_veggies: 5,
        }
    }

    pub fn plant(&mut self, to_plant: &mut VeggieQty) {
        if let Some(curr) = &mut self.veggies {
            curr.add(to_plant);
        } else {
            self.veggies = Some(to_plant.take().with_max(self.max_veggies));
        }
    }

    pub fn harvest(&mut self) -> Money {
        if let Some(veggies) = self.veggies.take() {
            veggies.value()
        } else {
            0
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Silo {
    pub storage: VecDeque<VeggieQty>,
    pub max_storage: usize,
}

impl Silo {
    pub fn new(rng: &mut SmallRng) -> Self {
        let mut barn = Silo {
            storage: VecDeque::new(),
            max_storage: 9,
        };

        barn.refill(rng);

        barn
    }

    pub fn refill(&mut self, rng: &mut SmallRng) {
        while self.storage.len() < self.max_storage {
            let veggies: Vec<_> = Veggie::iter().collect();
            let veggie = veggies.choose_weighted(rng, |v| v.qty()).unwrap();
            self.storage
                .push_back(VeggieQty::new(*veggie, 1).with_max(1))
        }
    }
}
