# BAR Rust — Feature Gap TODO

Comprehensive list of features BAR has that we don't, categorized by impact.
Current state: ~3,900 lines, single-player, 6 unit types, 6 buildings, basic combat.

---

## IMPACT: GAME-CHANGING (Transforms the game fundamentally)

### Multiplayer
- [ ] Networking layer (client-server or P2P lockstep)
- [ ] Lobby system (create/join games)
- [ ] Team support (allies, shared vision, resource sharing)
- [ ] Spectator mode
- [ ] Replay recording & playback
- [ ] Rankings / matchmaking

### Pathfinding
- [ ] A* or flow-field pathfinding (currently units beeline through obstacles)
- [ ] Terrain-aware pathing (avoid steep slopes, go around buildings/walls)
- [ ] Unit movement classes (bots climb hills, vehicles can't)
- [ ] Formation movement (Ctrl+right-click, units maintain group shape)

### Tech Tiers (T1/T2/T3)
- [ ] T2 factories (Advanced Bot Lab, Advanced Vehicle Plant)
- [ ] T2 units (heavy tanks, missile vehicles, advanced bots)
- [ ] T2 buildings (advanced extractors 4x output, advanced solar, fusion reactor)
- [ ] T3 / Experimental Gantry (endgame superunits)
- [ ] T1→T2 extractor upgrade mechanic (right-click with T2 constructor)
- [ ] Constructor units (T1/T2 builders from factories, not just commander)

### Second Faction (Cortex)
- [ ] Parallel unit roster with different stats/playstyle
- [ ] Cortex models (different visual identity)
- [ ] Faction-specific mechanics (armed extractors, cheaper solar, etc.)

---

## IMPACT: HIGH (Major gameplay depth additions)

### Air Units
- [ ] Aircraft Plant (T1 factory)
- [ ] Fighters (air-to-air combat)
- [ ] Bombers (line-drop 5 bombs)
- [ ] Gunships (hover + strafe)
- [ ] Air transports (carry units)
- [ ] Air constructors (flying builders)
- [ ] Air repair pads
- [ ] Anti-air defenses (missile towers, flak guns)
- [ ] Anti-air units (bots/vehicles with AA weapons)
- [ ] Aircraft movement model (flight, strafing runs, landing)

### Naval
- [ ] Shipyard (T1 factory)
- [ ] Corvettes, frigates, destroyers
- [ ] Submarines (sonar detection only)
- [ ] Water terrain (ocean, rivers, variable depth)
- [ ] Underwater metal extractors
- [ ] Naval defense buildings (torpedo launchers, floating turrets)
- [ ] Hovercraft (land + water movement)
- [ ] Amphibious units

### Nuclear Weapons
- [ ] ICBM launcher (Armageddon) — devastating area strike
- [ ] Anti-nuke system (Citadel) — intercepts ICBMs, stockpile missiles
- [ ] Nuclear terrain deformation (craters)
- [ ] Strategic gameplay layer (nuke threat forces expansion/defense)

### Proper AI Opponent
- [ ] AI economy management (build extractors, solar, factories)
- [ ] AI army composition (mixed unit types, not just tanks)
- [ ] AI attack strategies (raid, push, expand)
- [ ] AI defense building (walls, LLTs at chokepoints)
- [ ] AI scouting behavior
- [ ] AI difficulty levels (Easy/Medium/Hard)
- [ ] AI build order priorities

### Advanced Combat Mechanics
- [ ] Armor classes (commanders, standard, vtol, ships, subs)
- [ ] Weapon damage multipliers vs armor types
- [ ] Flanking bonus (attacks from multiple angles = up to 200% damage)
- [ ] Terrain height combat advantage (high ground = range bonus for ballistic weapons)
- [ ] Laser damage falloff (100% at 0 range → 50% at max range)
- [ ] AoE damage (plasma cannons, explosions with splash radius)
- [ ] Weapon types: plasma (ballistic arc), rocket (homing), lightning (chains to nearby)

---

## IMPACT: MEDIUM (Significant quality-of-life or tactical depth)

### Commands & Controls
- [ ] Attack-move / Fight command (F key — move but engage enemies en route)
- [ ] Patrol command (P key — loop waypoints, auto-engage/repair/reclaim)
- [ ] Guard command (follow and protect/assist a unit)
- [ ] Hold position stance
- [ ] Control groups (Ctrl+0-9 assign, 0-9 recall)
- [ ] Command queuing (Shift+click to queue orders)
- [ ] Stop command (S key — cancel all orders)
- [ ] Fire stance toggle (fire at will / hold fire / return fire)
- [ ] Area reclaim (drag circle to reclaim all in area)
- [ ] Area attack (Alt+A — artillery fires at ground position)
- [ ] Move line drag (right-click drag to set formation line)

### Economy Depth
- [ ] Energy-to-metal converters (energy → metal conversion)
- [ ] Resource storage buildings (increase metal/energy capacity caps)
- [ ] Resource capacity limits (overflow = waste)
- [ ] Metal/energy sharing between allies
- [ ] Build assist (multiple constructors speed up same project)
- [ ] Nano towers (stationary high-buildpower constructors)
- [ ] Build priority system (high-priority gets resources first)
- [ ] Wind turbines (variable output based on map)
- [ ] Geothermal plants (placed on geo vents, high energy)
- [ ] Fusion reactors (expensive, high energy output)

### Unit Veterancy / Experience
- [ ] XP gained from kills (based on cost of killed unit)
- [ ] Level 1 (3x cost killed): +10% damage, +10% health
- [ ] Level 2 (6x cost killed): +30% damage, +30% health
- [ ] Level 3 (9x cost killed): +100% health, +50% damage, HP regen
- [ ] Visual rank indicators on veteran units

### Stealth & Electronic Warfare
- [ ] Cloak (visual invisibility, still on radar, energy cost)
- [ ] Stealth (radar invisibility, still visible)
- [ ] Radar jamming (obscure area from enemy radar)
- [ ] Counter-intelligence (reveal cloaked units)
- [ ] Spy units (invisible to radar)

### Long-Range Artillery
- [ ] Long-range plasma cannon buildings (cross-map range)
- [ ] Ballistic trajectory (affected by terrain height)
- [ ] High/low trajectory toggle
- [ ] Radar targeting (inaccurate without direct vision)
- [ ] Targeting facility building (reduces radar inaccuracy)

### More Defensive Buildings
- [ ] Pop-up turrets (hidden until enemy approaches)
- [ ] Heavy laser tower (T2 defense)
- [ ] Plasma deflector / shield (deflects cannon shots)
- [ ] Fortification walls (T2 — blocks direct fire, only crushable by T3)
- [ ] Mines (cloaked, triggered by proximity)

### Construction QoL
- [ ] Build line (Shift-drag to build structures in a line)
- [ ] Build grid (Shift+Alt-drag for rectangular grids)
- [ ] Building rotation ([ and ] for 90° rotation)
- [ ] Build spacing controls (Z/X to adjust)

---

## IMPACT: LOW-MEDIUM (Polish, immersion, nice-to-have)

### Visual Effects
- [ ] Explosion particles (smoke, fire, debris, shockwaves)
- [ ] Weapon-specific projectile visuals (laser beams, plasma balls, missiles with trails)
- [ ] Muzzle flash effects
- [ ] Water rendering (reflections, waves)
- [ ] Unit destruction animations (not just instant despawn)
- [ ] Wreckage models (actual wrecked unit models, not brown cuboids)
- [ ] Build-up animation (wireframe → solid during construction)
- [ ] Bloom post-processing
- [ ] Screen-space ambient occlusion (SSAO)
- [ ] Better terrain textures (actual textures instead of vertex colors)
- [ ] Grass/vegetation rendering
- [ ] Particle effects for construction beams

### Audio
- [ ] Background music / soundtrack
- [ ] Combat sound effects (weapon fire, explosions, impacts)
- [ ] Unit response voices (acknowledgment, movement, attack)
- [ ] Ambient sounds (wind, environment)
- [ ] UI click sounds
- [ ] Construction sounds
- [ ] Alert notifications (commander under attack, etc.)

### UI Improvements
- [ ] Proper minimap with unit dots (colored by team)
- [ ] Resource bars with capacity indicators
- [ ] Unit info panel (selected unit stats, HP, damage, etc.)
- [ ] Build menu with icons (not just keyboard shortcuts)
- [ ] Factory production progress bar
- [ ] Tooltip system
- [ ] Game speed controls (pause, speed up, slow down)
- [ ] Options/settings menu
- [ ] Tab key: toggle between 3D and top-down map view
- [ ] Strategic zoom (zoom out to icon view)
- [ ] F4: Show metal spot locations overlay

### PvE Modes
- [ ] Scavengers (AI faction that escalates, spawns from burrows, boss system)
- [ ] Raptors (insect enemies from hives, queen boss)
- [ ] Difficulty scaling

### Map System
- [ ] Multiple maps (not just one hardcoded terrain)
- [ ] Map loading from files
- [ ] Map editor
- [ ] Variable map sizes
- [ ] Map features: geothermal vents, water bodies

### Miscellaneous Mechanics
- [ ] Self-destruct (Ctrl+D — destroy own unit, no wreckage, denies reclaim)
- [ ] Resurrection (special units can revive wreckage into working units)
- [ ] Unit capture
- [ ] Transportable structures (air transport picks up turrets/radar)
- [ ] Target set system (Y key — designate preferred target type)
- [ ] Wait command (W key — pause orders without clearing queue)
- [ ] Repeat queue toggle for factories (continuous production loop)

---

## Summary — What moves the needle most

| Priority | Feature | Why |
|----------|---------|-----|
| 1 | **Pathfinding** | Units walking through walls/buildings is the #1 jank |
| 2 | **Multiplayer** | Transforms from toy to real game, biggest draw |
| 3 | **Proper AI** | Single-player is boring if enemy just spams tanks |
| 4 | **Tech tiers (T2/T3)** | Gives progression, strategic depth, build variety |
| 5 | **Air units** | Opens entirely new combat dimension |
| 6 | **Commands (attack-move, patrol, control groups, queuing)** | Basic RTS playability |
| 7 | **Combat depth (armor, flanking, AoE, weapon types)** | Makes fights interesting |
| 8 | **Audio** | Silent game feels dead, huge immersion gap |
| 9 | **Constructor units + build assist** | Core BAR economy loop |
| 10 | **Economy depth (converters, storage, capacity)** | Adds strategic resource management |
| 11 | **Second faction** | Doubles content, asymmetric play |
| 12 | **Naval** | New map types, strategic layer |
| 13 | **Nukes** | Endgame drama, strategic tension |
| 14 | **UI overhaul** | Playability and polish |
| 15 | **Visual effects** | Makes combat feel impactful |
