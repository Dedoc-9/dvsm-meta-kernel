// ============================================================================
// DVSM / REAL ENGINE CORE (SINGLE FILE PRODUCTION BUILD)
// Deterministic ECS + Netcode + Rollback + Genesis + Validation
// Author: Daniel J. Dillberg
// ============================================================================

#![allow(non_camel_case_types)]
use std::collections::VecDeque;

// ============================================================================
// TYPES
// ============================================================================

pub type Entity = u32;
pub type Fx = i64; // fixed-point (logical deterministic scalar)

// ============================================================================
// FIXED UPDATE CONSTANT
// ============================================================================

const DT: Fx = 1;

// ============================================================================
// VECTOR
// ============================================================================

#[derive(Clone, Copy, Debug, Default)]
pub struct Vec3(pub Fx, pub Fx, pub Fx);

// ============================================================================
// ECS WORLD (STRUCT OF ARRAYS - DETERMINISTIC ORDER)
// ============================================================================

#[derive(Clone)]
pub struct World {
    pub entities: Vec<Entity>,
    pub pos: Vec<Vec3>,
    pub vel: Vec<Vec3>,
}

impl World {
    pub fn new() -> Self {
        Self {
            entities: vec![],
            pos: vec![],
            vel: vec![],
        }
    }

    #[inline(always)]
    pub fn spawn(&mut self, id: Entity) {
        self.entities.push(id);
        self.pos.push(Vec3(0, 0, 0));
        self.vel.push(Vec3(0, 0, 0));
    }

    #[inline(always)]
    pub fn index_of(&self, id: Entity) -> Option<usize> {
        self.entities.iter().position(|x| *x == id)
    }

    #[inline(always)]
    pub fn remove(&mut self, id: Entity) {
        if let Some(i) = self.index_of(id) {
            self.entities.swap_remove(i);
            self.pos.swap_remove(i);
            self.vel.swap_remove(i);
        }
    }

    // ------------------------------------------------------------
    // DETERMINISTIC PHYSICS STEP
    // ------------------------------------------------------------

    #[inline(always)]
    pub fn tick(&mut self) {
        for i in 0..self.entities.len() {
            self.pos[i].0 += self.vel[i].0 * DT;
            self.pos[i].1 += self.vel[i].1 * DT;
            self.pos[i].2 += self.vel[i].2 * DT;
        }
    }

    // ------------------------------------------------------------
    // STATE HASH (S_ECHO)
    // ------------------------------------------------------------

    pub fn s_echo(&self) -> u64 {
        let mut h: u64 = 0xcbf29ce484222325;

        for i in 0..self.entities.len() {
            h ^= self.entities[i] as u64;
            h = h.wrapping_mul(1099511628211);

            h ^= self.pos[i].0 as u64;
            h = h.wrapping_mul(1099511628211);

            h ^= self.pos[i].1 as u64;
            h = h.wrapping_mul(1099511628211);

            h ^= self.pos[i].2 as u64;
            h = h.wrapping_mul(1099511628211);
        }

        h
    }
}

// ============================================================================
// COMMAND SYSTEM (DETERMINISTIC INPUT LAYER)
// ============================================================================

#[derive(Clone)]
pub enum Command {
    Spawn(Entity),
    Destroy(Entity),
    SetVel(Entity, Vec3),
}

// ============================================================================
// SNAPSHOT (ROLLBACK SYSTEM)
// ============================================================================

#[derive(Clone)]
pub struct Snapshot {
    pub tick: u64,
    pub world: World,
    pub echo: u64,
}

// ============================================================================
// SERVER CORE
// ============================================================================

pub struct Server {
    pub world: World,
    pub tick: u64,
    pub buffer: Vec<Command>,
}

impl Server {
    pub fn new() -> Self {
        Self {
            world: World::new(),
            tick: 0,
            buffer: vec![],
        }
    }

    // ------------------------------------------------------------
    // APPLY COMMANDS (DETERMINISTIC ORDER ONLY)
    // ------------------------------------------------------------

    fn apply(&mut self, cmd: Command) {
        match cmd {
            Command::Spawn(id) => self.world.spawn(id),
            Command::Destroy(id) => self.world.remove(id),
            Command::SetVel(id, v) => {
                if let Some(i) = self.world.index_of(id) {
                    self.world.vel[i] = v;
                }
            }
        }
    }

    // ------------------------------------------------------------
    // MAIN TICK
    // ------------------------------------------------------------

    pub fn step(&mut self) -> u64 {
        self.tick += 1;

        let cmds = std::mem::take(&mut self.buffer);
        for c in cmds {
            self.apply(c);
        }

        self.world.tick();
        self.world.s_echo()
    }
}

// ============================================================================
// CLIENT (PREDICTION + ROLLBACK)
// ============================================================================

pub struct Client {
    pub world: World,
    pub history: VecDeque<Snapshot>,
    pub last_echo: u64,
}

impl Client {
    pub fn new(world: World) -> Self {
        Self {
            world,
            history: VecDeque::new(),
            last_echo: 0,
        }
    }

    // ------------------------------------------------------------
    // SAVE SNAPSHOT (RING BUFFER)
    // ------------------------------------------------------------

    pub fn snapshot(&mut self, tick: u64, echo: u64) {
        self.history.push_back(Snapshot {
            tick,
            world: self.world.clone(),
            echo,
        });

        if self.history.len() > 64 {
            self.history.pop_front();
        }
    }

    // ------------------------------------------------------------
    // ROLLBACK
    // ------------------------------------------------------------

    pub fn rollback(&mut self, tick: u64) {
        if let Some(snap) = self.history.iter().rev().find(|s| s.tick <= tick) {
            self.world = snap.world.clone();
        }
    }

    // ------------------------------------------------------------
    // RESIMULATION LOOP
    // ------------------------------------------------------------

    pub fn resimulate(&mut self, from: u64) {
        self.rollback(from);

        for _ in from..self.history.back().map(|s| s.tick).unwrap_or(from) {
            self.world.tick();
        }
    }

    // ------------------------------------------------------------
    // RECONCILIATION
    // ------------------------------------------------------------

    pub fn reconcile(&mut self, server_echo: u64, tick: u64) {
        if server_echo != self.last_echo {
            self.rollback(tick);
            self.resimulate(tick);
        }

        self.last_echo = server_echo;
    }
}

// ============================================================================
// NETWORK MODEL (LOGICAL FRAME FORMAT)
// ============================================================================

pub enum NetFrame {
    Input {
        tick: u64,
        entity: Entity,
        vel: Vec3,
    },
    Snapshot {
        tick: u64,
        echo: u64,
    },
}

// ============================================================================
// DELTA COMPRESSION (MINIMAL BITPACK STUB)
// ============================================================================

pub fn compress_delta(prev: &World, curr: &World) -> Vec<u8> {
    let mut out = vec![];

    out.push(curr.entities.len() as u8);

    for i in 0..curr.entities.len().min(prev.entities.len()) {
        let dx = curr.pos[i].0 - prev.pos[i].0;
        let dy = curr.pos[i].1 - prev.pos[i].1;
        let dz = curr.pos[i].2 - prev.pos[i].2;

        out.extend_from_slice(&dx.to_le_bytes());
        out.extend_from_slice(&dy.to_le_bytes());
        out.extend_from_slice(&dz.to_le_bytes());
    }

    out
}

// ============================================================================
// GENESIS (S₀ STATE)
// ============================================================================

pub fn genesis() -> World {
    let mut w = World::new();
    w.spawn(1);
    w.spawn(2);
    w
}

pub fn genesis_hash(w: &World) -> u64 {
    w.s_echo()
}

// ============================================================================
// DETERMINISM TEST (INLINE VALIDATION)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_hash() {
        let mut a = World::new();
        let mut b = World::new();

        a.spawn(1);
        b.spawn(1);

        for _ in 0..100 {
            a.tick();
            b.tick();
        }

        assert_eq!(a.s_echo(), b.s_echo());
    }

    #[test]
    fn rollback_consistency() {
        let w = genesis();
        let mut c = Client::new(w.clone());

        let mut s = Server::new();
        s.world = w;

        for i in 0..10 {
            s.step();
            c.world.tick();

            let echo = s.world.s_echo();
            c.snapshot(i, echo);
            c.reconcile(echo, i);
        }
    }
}

// ============================================================================
// END SINGLE FILE ENGINE CORE
// ============================================================================
