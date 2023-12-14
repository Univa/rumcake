//! Layout management.

/// A procedural macro to generate [Layers](type.Layers.html)
/// ## Syntax
/// Items inside the macro are converted to Actions as such:
/// - [`Action::KeyCode`]: Idents are automatically understood as keycodes: `A`, `RCtrl`, `Space`
///     - Punctuation, numbers and other literals that aren't special to the rust parser are converted
///       to KeyCodes as well: `,` becomes `KeyCode::Commma`, `2` becomes `KeyCode::Kb2`, `/` becomes `KeyCode::Slash`
///     - Characters which require shifted keys are converted to `Action::MultipleKeyCodes(&[LShift, <character>])`:
///       `!` becomes `Action::MultipleKeyCodes(&[LShift, Kb1])` etc
///     - Characters special to the rust parser (parentheses, brackets, braces, quotes, apostrophes, underscores, backslashes and backticks)
///       left alone cause parsing errors and as such have to be enclosed by apostrophes: `'['` becomes `KeyCode::LBracket`,
///       `'\''` becomes `KeyCode::Quote`, `'\\'` becomes `KeyCode::BSlash`
/// - [`Action::NoOp`]: Lowercase `n`
/// - [`Action::Trans`]: Lowercase `t`
/// - [`Action::Layer`]: A number in parentheses: `(1)`, `(4 - 2)`, `(0x4u8 as usize)`
/// - [`Action::MultipleActions`]: Actions in brackets: `[LCtrl S]`, `[LAlt LCtrl C]`, `[(2) B {Action::NoOp}]`
/// - Other `Action`s: anything in braces (`{}`) is copied unchanged to the final layout - `{ Action::Custom(42) }`
///   simply becomes `Action::Custom(42)`
///
/// **Important note**: comma (`,`) is a keycode on its own, and can't be used to separate keycodes as one would have
/// to do when not using a macro.
///
/// ## Usage example:
/// Example layout for a 12x4 split keyboard:
/// ```
/// use keyberon::action::Action;
/// use keyberon::layout::Layers;
/// static DLAYER: Action = Action::DefaultLayer(5);
///
/// pub static LAYERS: Layers<12, 4, 2> = keyberon::layout::layout! {
///     {
///         [ Tab    Q W E R T   Y U I O P BSpace ]
///         [ LCtrl  A S D F G   H J K L ; Quote  ]
///         [ LShift Z X C V B   N M , . / Escape ]
///         [ n n LGui {DLAYER} Space Escape   BSpace Enter (1) RAlt n n ]
///     }
///     {
///         [ Tab    1 2 3 4 5   6 7 8 9 0 BSpace  ]
///         [ LCtrl  ! @ # $ %   ^ & * '(' ')' -   ]
///         [ LShift n n n n n   n n n n n [LAlt A]]
///         [ n n LGui (2) t t   t t t RAlt n n    ]
///     }
///     // ...
/// };
/// ```
pub use keyberon_macros::*;

use crate::action::{
    Action, HoldTapAction, HoldTapConfig, OneShotAction, OneShotEndConfig, SequenceEvent,
    TapDanceAction, TapDanceConfig,
};
use crate::key_code::KeyCode;
use arraydeque::ArrayDeque;
use heapless::Vec;

use State::*;

/// The Layers type.
///
/// `Layers` type is an array of layers which contain the description
/// of actions on the switch matrix. For example `layers[1][2][3]`
/// corresponds to the key on the first layer, row 2, column 3.
/// The generic parameters are in order: the number of columns, rows and layers,
/// and the type contained in custom actions.
pub type Layers<
    const C: usize,
    const R: usize,
    const L: usize,
    T = core::convert::Infallible,
    K = KeyCode,
> = [[[Action<T, K>; C]; R]; L];

/// The current event stack.
///
/// Events can be retrieved by iterating over this struct and calling [Stacked::event].
type Stack = ArrayDeque<[Stacked; 16], arraydeque::behavior::Wrapping>;

/// The layout manager. It takes `Event`s and `tick`s as input, and
/// generate keyboard reports.
pub struct Layout<
    const C: usize,
    const R: usize,
    const L: usize,
    T = core::convert::Infallible,
    K = KeyCode,
> where
    T: 'static + Copy,
    K: 'static + Copy,
{
    layers: &'static mut [[[Action<T, K>; C]; R]; L],
    default_layer: usize,
    states: Vec<State<T, K>, 64>,
    waiting: Option<WaitingState<T, K>>,
    oneshot: Option<OneShotState>,
    tapdance: Option<TapDanceState<T, K>>,
    active_sequences: ArrayDeque<[SequenceState<K>; 4], arraydeque::behavior::Wrapping>,
    stacked: Stack,
    tap_hold_tracker: TapHoldTracker,
}

/// An event on the key matrix.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Event {
    /// Press event with coordinates (i, j).
    Press(u8, u8),
    /// Release event with coordinates (i, j).
    Release(u8, u8),
}
impl Event {
    /// Returns the coordinates (i, j) of the event.
    pub fn coord(self) -> (u8, u8) {
        match self {
            Event::Press(i, j) => (i, j),
            Event::Release(i, j) => (i, j),
        }
    }

    /// Transforms the coordinates of the event.
    ///
    /// # Example
    ///
    /// ```
    /// # use keyberon::layout::Event;
    /// assert_eq!(
    ///     Event::Press(3, 10),
    ///     Event::Press(3, 1).transform(|i, j| (i, 11 - j)),
    /// );
    /// ```
    pub fn transform(self, f: impl FnOnce(u8, u8) -> (u8, u8)) -> Self {
        match self {
            Event::Press(i, j) => {
                let (i, j) = f(i, j);
                Event::Press(i, j)
            }
            Event::Release(i, j) => {
                let (i, j) = f(i, j);
                Event::Release(i, j)
            }
        }
    }

    /// Returns `true` if the event is a key press.
    pub fn is_press(self) -> bool {
        match self {
            Event::Press(..) => true,
            Event::Release(..) => false,
        }
    }

    /// Returns `true` if the event is a key release.
    pub fn is_release(self) -> bool {
        match self {
            Event::Release(..) => true,
            Event::Press(..) => false,
        }
    }
}

/// Event from custom action.
#[derive(Debug, PartialEq, Eq, Default)]
pub enum CustomEvent<T: Copy> {
    /// No custom action.
    #[default]
    NoEvent,
    /// The given custom action key is pressed.
    Press(T),
    /// The given custom action key is released.
    Release(T),
}
impl<T: Copy> CustomEvent<T> {
    /// Update an event according to a new event.
    ///
    ///The event can only be modified in the order `NoEvent < Press <
    /// Release`
    fn update(&mut self, e: Self) {
        use CustomEvent::*;
        match (&e, &self) {
            (Release(_), NoEvent) | (Release(_), Press(_)) => *self = e,
            (Press(_), NoEvent) => *self = e,
            _ => (),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
enum State<T: 'static + Copy, K: 'static + Copy> {
    NormalKey { keycode: K, coord: (u8, u8) },
    FakeKey { keycode: K },
    MomentaryLayerModifier { value: usize, coord: (u8, u8) },
    ToggleLayerModifier { value: usize },
    Custom { value: T, coord: (u8, u8) },
}
impl<T: 'static + Copy, K: 'static + Copy> Copy for State<T, K> {}
impl<T: 'static + Copy, K: 'static + Copy> Clone for State<T, K> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: 'static + Copy, K: 'static + Copy + PartialEq> State<T, K> {
    fn keycode(&self) -> Option<K> {
        match self {
            NormalKey { keycode, .. } => Some(*keycode),
            FakeKey { keycode } => Some(*keycode),
            _ => None,
        }
    }
    fn tick(&self) -> Option<Self> {
        Some(*self)
    }
    fn release(&self, c: (u8, u8), custom: &mut CustomEvent<T>) -> Option<Self> {
        match *self {
            NormalKey { coord, .. } | MomentaryLayerModifier { coord, .. } if coord == c => None,
            Custom { value, coord } if coord == c => {
                custom.update(CustomEvent::Release(value));
                None
            }
            _ => Some(*self),
        }
    }
    fn sequence_release(&self, key: K) -> Option<Self> {
        match *self {
            FakeKey { keycode } if keycode == key => None,
            _ => Some(*self),
        }
    }
    fn get_layer(&self) -> Option<usize> {
        match self {
            MomentaryLayerModifier { value, .. } => Some(*value),
            ToggleLayerModifier { value, .. } => Some(*value),
            _ => None,
        }
    }
}

#[derive(Debug)]
struct WaitingState<T: 'static, K: 'static> {
    coord: (u8, u8),
    timeout: u16,
    delay: u16,
    hold: &'static Action<T, K>,
    tap: &'static Action<T, K>,
    config: HoldTapConfig,
}

/// Actions that can be triggered for a key configured for HoldTap.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum WaitingAction {
    /// Trigger the holding event.
    Hold,
    /// Trigger the tapping event.
    Tap,
    /// Drop this event. It will act as if no key was pressed.
    NoOp,
}

impl<T, K> WaitingState<T, K> {
    fn tick(&mut self, stacked: &Stack) -> Option<WaitingAction> {
        self.timeout = self.timeout.saturating_sub(1);
        match self.config {
            HoldTapConfig::Default => (),
            HoldTapConfig::HoldOnOtherKeyPress => {
                if stacked.iter().any(|s| s.event.is_press()) {
                    return Some(WaitingAction::Hold);
                }
            }
            HoldTapConfig::PermissiveHold => {
                for (x, s) in stacked.iter().enumerate() {
                    if s.event.is_press() {
                        let (i, j) = s.event.coord();
                        let target = Event::Release(i, j);
                        if stacked.iter().skip(x + 1).any(|s| s.event == target) {
                            return Some(WaitingAction::Hold);
                        }
                    }
                }
            }
            HoldTapConfig::Custom(func) => {
                if let waiting_action @ Some(_) = (func)(StackedIter(stacked.iter())) {
                    return waiting_action;
                }
            }
        }
        if let Some(&Stacked { since, .. }) = stacked
            .iter()
            .find(|s| self.is_corresponding_release(&s.event))
        {
            if self.timeout + since > self.delay {
                Some(WaitingAction::Tap)
            } else {
                Some(WaitingAction::Hold)
            }
        } else if self.timeout == 0 {
            Some(WaitingAction::Hold)
        } else {
            None
        }
    }
    fn is_corresponding_release(&self, event: &Event) -> bool {
        matches!(event, Event::Release(i, j) if (*i, *j) == self.coord)
    }
}

struct SequenceState<K>
where
    K: 'static + Copy,
{
    events: &'static [SequenceEvent<K>],
    current_event: u8,
    delay: u16,
    tap_in_progress: bool,
}

struct TapDanceState<T, K>
where
    T: 'static,
    K: 'static,
{
    coord: (u8, u8),
    timeout: u16,
    current_action: u8,
    tapdance_action: &'static TapDanceAction<T, K>,
    release_on_tapdance_end: bool, // This is only used for lazy config
}

impl<T, K> TapDanceState<T, K> {
    fn tick(&mut self) {
        self.timeout = self.timeout.saturating_sub(1);
    }

    fn handle_press(&mut self, corresponds_to_current_tapdance: bool) {
        if corresponds_to_current_tapdance {
            self.current_action += 1;
            self.timeout = self.tapdance_action.timeout;
            self.release_on_tapdance_end = false;
            return;
        }

        self.timeout = 0;
    }

    fn handle_release(&mut self, coord: (u8, u8)) -> bool {
        if coord == self.coord {
            self.release_on_tapdance_end = true;
            return true;
        }

        false
    }

    fn is_done(&self) -> bool {
        self.timeout == 0 || self.tapdance_action.actions.len() as u8 == self.current_action + 1
    }
}

struct OneShotState {
    /// KCoordinates of one shot keys that are active
    active_oneshot_keys: ArrayDeque<[(u8, u8); 16], arraydeque::behavior::Wrapping>,
    /// KCoordinates of one shot keys that have been released. The release events will be
    /// registered at the end of the oneshot action.
    released_oneshot_keys: ArrayDeque<[(u8, u8); 16], arraydeque::behavior::Wrapping>,
    /// Used to keep track of already-pressed keys for the release variants.
    other_pressed_keys: ArrayDeque<[(u8, u8); 16], arraydeque::behavior::Wrapping>,
    /// Timeout (ms) after which all one shot keys expire
    timeout: u16,
    /// Contains the end config of the most recently pressed one shot key
    end_config: OneShotEndConfig,
    /// Marks if release of the one shot keys should be done on the next tick
    release_on_next_tick: bool,
}

impl OneShotState {
    fn tick(&mut self) -> Option<Vec<(u8, u8), 16>> {
        if self.active_oneshot_keys.is_empty() {
            return None;
        }

        self.timeout = self.timeout.saturating_sub(1);

        if !self.release_on_next_tick && self.timeout > 0 {
            return None;
        }

        self.active_oneshot_keys.clear();
        self.other_pressed_keys.clear();
        Some(self.released_oneshot_keys.drain(..).collect())
    }

    fn handle_press(&mut self, coord: (u8, u8), is_oneshot_activation: bool) -> Option<(u8, u8)> {
        // If a key was pressed, and shares the same coordinates as an active oneshot key that was
        // previously released, we need to remove it from [`self.released_oneshot_keys`], otherwise
        // it will be immediately released with the [`OneShotEndConfig::EndOnFirstPress`] config
        // selected.
        self.released_oneshot_keys.retain(|c| *c != coord);
        match is_oneshot_activation {
            true => {
                if matches!(
                    self.end_config,
                    OneShotEndConfig::EndOnFirstReleaseOrRepress
                        | OneShotEndConfig::EndOnFirstPressOrRepress
                ) && self.active_oneshot_keys.contains(&coord)
                {
                    self.release_on_next_tick = true;
                }
                self.active_oneshot_keys.push_back(coord)
            }
            false => {
                if matches!(
                    self.end_config,
                    OneShotEndConfig::EndOnFirstPress | OneShotEndConfig::EndOnFirstPressOrRepress
                ) {
                    self.release_on_next_tick = true;
                } else {
                    self.other_pressed_keys.push_back(coord);
                }
                None
            }
        }
    }

    fn handle_release(&mut self, coord: (u8, u8)) -> (bool, Option<Vec<(u8, u8), 16>>) {
        if matches!(
            self.end_config,
            OneShotEndConfig::EndOnFirstRelease | OneShotEndConfig::EndOnFirstReleaseOrRepress
        ) && self.other_pressed_keys.contains(&coord)
        {
            // Instead of releasing on next tick, we release the keys we want to release immediately
            return (false, Some(self.released_oneshot_keys.drain(..).collect()));
        }

        // Ignore releases for oneshot keys
        // They will be released at the end of the one shot.
        if self.active_oneshot_keys.contains(&coord) {
            return (
                true,
                self.released_oneshot_keys.push_back(coord).map(|overflow| {
                    let mut vec = Vec::new();
                    let _ = vec.push(overflow);
                    vec
                }),
            );
        }

        (false, None)
    }
}

/// An iterator over the currently stacked events.
///
/// Events can be retrieved by iterating over this struct and calling [Stacked::event].
pub struct StackedIter<'a>(arraydeque::Iter<'a, Stacked>);

impl<'a> Iterator for StackedIter<'a> {
    type Item = &'a Stacked;
    fn next(&mut self) -> Option<Self::Item> {
        self.0.next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.0.size_hint()
    }
}

/// An event, waiting in a stack to be processed.
#[derive(Debug)]
pub struct Stacked {
    event: Event,
    since: u16,
}
impl From<Event> for Stacked {
    fn from(event: Event) -> Self {
        Stacked { event, since: 0 }
    }
}
impl Stacked {
    fn tick(&mut self) {
        self.since = self.since.saturating_add(1);
    }

    /// Get the [Event] from this object.
    pub fn event(&self) -> Event {
        self.event
    }
}

#[derive(Default)]
struct TapHoldTracker {
    coord: (u8, u8),
    timeout: u16,
}

impl TapHoldTracker {
    fn tick(&mut self) {
        self.timeout = self.timeout.saturating_sub(1);
    }
}

/// Errors that can occur when attempting to change an action
pub enum ChangeActionError {
    /// The provided coordinates (layer, row and column) do not exist in the layout.
    OutOfBounds,
}

#[derive(Default)]
struct ActionContext {
    inside_oneshot: bool,
    inside_tapdance: bool,
}

impl<
        const C: usize,
        const R: usize,
        const L: usize,
        T: 'static + Copy,
        K: 'static + Copy + PartialEq,
    > Layout<C, R, L, T, K>
{
    /// Creates a new `Layout` object.
    pub fn new(layers: &'static mut [[[Action<T, K>; C]; R]; L]) -> Self {
        Self {
            layers,
            default_layer: 0,
            states: Vec::new(),
            waiting: None,
            oneshot: None,
            tapdance: None,
            active_sequences: ArrayDeque::new(),
            stacked: ArrayDeque::new(),
            tap_hold_tracker: Default::default(),
        }
    }
    /// Iterates on the key codes of the current state.
    pub fn keycodes(&self) -> impl Iterator<Item = K> + '_ {
        self.states.iter().filter_map(State::keycode)
    }
    fn waiting_into_hold(&mut self) -> CustomEvent<T> {
        if let Some(w) = &self.waiting {
            let hold = w.hold;
            let coord = w.coord;
            self.waiting = None;
            if coord == self.tap_hold_tracker.coord {
                self.tap_hold_tracker.timeout = 0;
            }
            self.do_action(*hold, coord, 0, &mut ActionContext::default())
        } else {
            CustomEvent::NoEvent
        }
    }
    fn waiting_into_tap(&mut self) -> CustomEvent<T> {
        if let Some(w) = &self.waiting {
            let tap = w.tap;
            let coord = w.coord;
            self.waiting = None;
            self.do_action(*tap, coord, 0, &mut ActionContext::default())
        } else {
            CustomEvent::NoEvent
        }
    }
    fn drop_waiting(&mut self) -> CustomEvent<T> {
        self.waiting = None;
        CustomEvent::NoEvent
    }
    fn do_tapdance_action_if_lazy(&mut self, context: &mut ActionContext) -> CustomEvent<T> {
        if let Some(tapdance_state) = &mut self.tapdance {
            if matches!(tapdance_state.tapdance_action.config, TapDanceConfig::Lazy)
                && tapdance_state.is_done()
            {
                let action =
                    tapdance_state.tapdance_action.actions[tapdance_state.current_action as usize];
                let coord = tapdance_state.coord;
                let release = tapdance_state.release_on_tapdance_end;

                context.inside_tapdance = true;
                let custom = self.do_action(action, coord, 0, context);
                context.inside_tapdance = false;

                if release {
                    self.event(Event::Release(coord.0, coord.1))
                }

                self.tapdance = None;

                return custom;
            }
        }

        CustomEvent::NoEvent
    }
    fn do_tapdance_action_if_eager(&mut self, context: &mut ActionContext) -> CustomEvent<T> {
        if let Some(tapdance_state) = &mut self.tapdance {
            if matches!(tapdance_state.tapdance_action.config, TapDanceConfig::Eager) {
                let action =
                    tapdance_state.tapdance_action.actions[tapdance_state.current_action as usize];
                let coord = tapdance_state.coord;
                let done = tapdance_state.is_done();

                context.inside_tapdance = true;
                let custom = self.do_action(action, coord, 0, context);
                context.inside_tapdance = false;

                if done {
                    self.tapdance = None
                }

                return custom;
            }
        };

        CustomEvent::NoEvent
    }
    fn process_sequences(&mut self) {
        for _ in 0..self.active_sequences.len() {
            if let Some(mut sequence) = self.active_sequences.pop_front() {
                if sequence.delay > 0 {
                    sequence.delay -= 1;
                    self.active_sequences.push_back(sequence);
                    continue;
                }

                let event = sequence.events.get(sequence.current_event as usize);

                match event {
                    Some(&SequenceEvent::NoOp) => {
                        sequence.current_event += 1;
                    }
                    Some(&SequenceEvent::Press(keycode)) => {
                        let _ = self.states.push(FakeKey { keycode });
                        sequence.current_event += 1;
                    }
                    Some(&SequenceEvent::Release(keycode)) => {
                        self.states
                            .retain(|s| s.sequence_release(keycode).is_some());
                        sequence.current_event += 1;
                    }
                    Some(&SequenceEvent::Tap(keycode)) if !sequence.tap_in_progress => {
                        sequence.tap_in_progress = true;
                        let _ = self.states.push(FakeKey { keycode });
                    }
                    Some(&SequenceEvent::Tap(keycode)) => {
                        sequence.tap_in_progress = false;
                        self.states
                            .retain(|s| s.sequence_release(keycode).is_some());
                        sequence.current_event += 1;
                    }
                    Some(&SequenceEvent::Delay(delay)) => {
                        sequence.delay = delay.saturating_sub(1);
                        sequence.current_event += 1;
                    }
                    Some(&SequenceEvent::Complete) => {
                        for state in self.states.clone().iter() {
                            if let FakeKey { keycode } = state {
                                self.states
                                    .retain(|s| s.sequence_release(*keycode).is_some());
                            }
                        }
                        // skip to the end of the sequence
                        sequence.current_event = sequence.events.len() as u8;
                    }
                    None => {}
                }

                // if the sequence is not done (an event was processed, and there are still events
                // to process), add it back to the list of active sequences
                if event.is_some() && sequence.current_event < sequence.events.len() as u8 {
                    self.active_sequences.push_back(sequence);
                }
            }
        }
    }
    /// A time event.
    ///
    /// This method must be called regularly, typically every millisecond.
    ///
    /// Returns the corresponding `CustomEvent`, allowing to manage
    /// custom actions thanks to the `Action::Custom` variant.
    pub fn tick(&mut self) -> CustomEvent<T> {
        self.states = self.states.iter().filter_map(State::tick).collect();
        self.stacked.iter_mut().for_each(Stacked::tick);
        self.tap_hold_tracker.tick();

        let mut custom = CustomEvent::NoEvent;
        let mut should_unstack = true;

        // process sequences
        self.process_sequences();

        // process oneshot
        if let Some(oneshot) = &mut self.oneshot {
            if let Some(to_release) = oneshot.tick() {
                for (i, j) in to_release {
                    custom.update(self.unstack(Stacked {
                        event: Event::Release(i, j),
                        since: 0,
                    }));
                }
                self.oneshot = None
            }
        }

        // process tap dance
        if let Some(tapdance) = &mut self.tapdance {
            tapdance.tick();

            if matches!(tapdance.tapdance_action.config, TapDanceConfig::Lazy) {
                custom.update(self.do_tapdance_action_if_lazy(&mut ActionContext::default()));
            } else if tapdance.is_done() {
                self.tapdance = None
            }
        }

        // process hold tap
        if let Some(w) = &mut self.waiting {
            should_unstack = false;
            custom.update(match w.tick(&self.stacked) {
                Some(WaitingAction::Hold) => self.waiting_into_hold(),
                Some(WaitingAction::Tap) => self.waiting_into_tap(),
                Some(WaitingAction::NoOp) => self.drop_waiting(),
                None => CustomEvent::NoEvent,
            });
        }

        // process normally
        if should_unstack {
            custom.update(match self.stacked.pop_front() {
                Some(s) => self.unstack(s),
                None => CustomEvent::NoEvent,
            })
        }

        custom
    }
    fn unstack(&mut self, stacked: Stacked) -> CustomEvent<T> {
        use Event::*;
        match stacked.event {
            Release(i, j) => {
                let mut custom = CustomEvent::NoEvent;
                let mut should_release_normally = true;

                if let Some(oneshot) = &mut self.oneshot {
                    let (ignore_release, extra_releases) = oneshot.handle_release((i, j));

                    if let Some(to_release) = extra_releases {
                        for (i2, j2) in to_release {
                            self.states
                                .retain(|s| s.release((i2, j2), &mut custom).is_some());
                        }

                        // If this is a non-oneshot key release, but there are extra releases, then
                        // that means the extra releases are pressed oneshot keys and that this
                        // oneshot is complete.
                        if !ignore_release {
                            self.oneshot = None
                        }
                    }

                    if ignore_release {
                        should_release_normally = false
                    }
                }

                if let Some(tapdance_state) = &mut self.tapdance {
                    tapdance_state.handle_release((i, j));
                }

                if should_release_normally {
                    self.states
                        .retain(|s| s.release((i, j), &mut custom).is_some());
                }

                custom
            }
            Press(i, j) => {
                let action = self.press_as_action((i, j), self.current_layer());
                self.do_action(action, (i, j), stacked.since, &mut ActionContext::default())
            }
        }
    }
    /// Register a key event.
    pub fn event(&mut self, event: Event) {
        if let Some(stacked) = self.stacked.push_back(event.into()) {
            self.waiting_into_hold();
            self.unstack(stacked);
        }
    }
    fn press_as_action(&self, coord: (u8, u8), layer: usize) -> Action<T, K> {
        use crate::action::Action::*;
        let action = self
            .layers
            .get(layer)
            .and_then(|l| l.get(coord.0 as usize))
            .and_then(|l| l.get(coord.1 as usize));
        match action {
            None => NoOp,
            Some(Trans) => {
                if layer != self.default_layer {
                    self.press_as_action(coord, self.default_layer)
                } else {
                    NoOp
                }
            }
            Some(&action) => action,
        }
    }
    /// Changes the action for a given key
    pub fn change_action(
        &mut self,
        coord: (u8, u8),
        layer: usize,
        action: Action<T, K>,
    ) -> Result<(), ChangeActionError> {
        self.layers
            .get_mut(layer)
            .and_then(|l| l.get_mut(coord.0 as usize))
            .and_then(|l| l.get_mut(coord.1 as usize))
            .map(|a| *a = action)
            .ok_or(ChangeActionError::OutOfBounds)
    }
    /// Get a copy of the action for a given key
    pub fn get_action(&mut self, coord: (u8, u8), layer: usize) -> Option<Action<T, K>> {
        self.layers
            .get(layer)
            .and_then(|l| l.get(coord.0 as usize))
            .and_then(|l| l.get(coord.1 as usize))
            .copied()
    }
    fn do_action(
        &mut self,
        action: Action<T, K>,
        coord: (u8, u8),
        delay: u16,
        context: &mut ActionContext,
    ) -> CustomEvent<T> {
        assert!(self.waiting.is_none());
        use Action::*;
        match action {
            NoOp | Trans => {
                self.handle_terminal_action(coord, context);
            }
            HoldTap(HoldTapAction {
                timeout,
                hold,
                tap,
                config,
                tap_hold_interval,
            }) => {
                if *tap_hold_interval == 0
                    || coord != self.tap_hold_tracker.coord
                    || self.tap_hold_tracker.timeout == 0
                {
                    let waiting: WaitingState<T, K> = WaitingState {
                        coord,
                        timeout: *timeout,
                        delay,
                        hold,
                        tap,
                        config: *config,
                    };
                    self.waiting = Some(waiting);
                    self.tap_hold_tracker.timeout = *tap_hold_interval;
                } else {
                    self.tap_hold_tracker.timeout = 0;
                    self.do_action(*tap, coord, delay, context);
                }
                // Need to set tap_hold_tracker coord AFTER the checks.
                self.tap_hold_tracker.coord = coord;
            }
            OneShot(&OneShotAction {
                action,
                timeout,
                end_config,
            }) => {
                self.tap_hold_tracker.coord = coord;
                context.inside_oneshot = true;
                let custom = self.do_action(action, coord, delay, context);
                context.inside_oneshot = false;
                let overflow = if let Some(oneshot) = &mut self.oneshot {
                    oneshot.end_config = end_config;
                    oneshot.timeout = timeout;
                    oneshot.handle_press(coord, true)
                } else {
                    let mut oneshot = OneShotState {
                        active_oneshot_keys: ArrayDeque::new(),
                        released_oneshot_keys: ArrayDeque::new(),
                        other_pressed_keys: ArrayDeque::new(),
                        timeout,
                        end_config,
                        release_on_next_tick: false,
                    };
                    let overflow = oneshot.handle_press(coord, true);
                    self.oneshot = Some(oneshot);
                    overflow
                };
                if let Some((i, j)) = overflow {
                    self.event(Event::Release(i, j))
                }
                return custom;
            }
            TapDance(tapdance_action) => {
                self.tap_hold_tracker.coord = coord;
                let mut custom = if let Some(tapdance_state) = &mut self.tapdance {
                    if coord == tapdance_state.coord
                        && core::ptr::eq(
                            tapdance_state.tapdance_action as *const _,
                            tapdance_action as *const _,
                        )
                    {
                        tapdance_state.handle_press(true);
                        return match tapdance_state.tapdance_action.config {
                            TapDanceConfig::Eager => self.do_tapdance_action_if_eager(context),
                            TapDanceConfig::Lazy => self.do_tapdance_action_if_lazy(context),
                        };
                    } else {
                        // end the current tap dance
                        tapdance_state.handle_press(false);
                        self.do_tapdance_action_if_lazy(context)
                    }
                } else {
                    CustomEvent::NoEvent
                };

                self.tapdance = Some(TapDanceState {
                    coord,
                    timeout: tapdance_action.timeout,
                    current_action: 0,
                    tapdance_action,
                    release_on_tapdance_end: false,
                });

                custom.update(match tapdance_action.config {
                    TapDanceConfig::Eager => self.do_tapdance_action_if_eager(context),
                    TapDanceConfig::Lazy => self.do_tapdance_action_if_lazy(context),
                });

                return custom;
            }
            Sequence(events) => {
                self.active_sequences.push_back(SequenceState {
                    events,
                    current_event: 0,
                    delay: 0,
                    tap_in_progress: false,
                });
            }
            KeyCode(keycode) => {
                self.tap_hold_tracker.coord = coord;
                let _ = self.states.push(NormalKey { coord, keycode });
                self.handle_terminal_action(coord, context);
            }
            MultipleKeyCodes(v) => {
                self.tap_hold_tracker.coord = coord;
                for &keycode in *v {
                    let _ = self.states.push(NormalKey { coord, keycode });
                }
                self.handle_terminal_action(coord, context);
            }
            MultipleActions(v) => {
                self.tap_hold_tracker.coord = coord;
                let mut custom = CustomEvent::NoEvent;
                for action in *v {
                    custom.update(self.do_action(*action, coord, delay, context));
                }
                return custom;
            }
            Layer(value) => {
                self.tap_hold_tracker.coord = coord;
                let _ = self.states.push(MomentaryLayerModifier { value, coord });
                self.handle_terminal_action(coord, context);
            }
            ToggleLayer(value) => {
                self.tap_hold_tracker.coord = coord;
                let mut removed = false;
                self.states.retain(|s| {
                    if matches!(s, ToggleLayerModifier { value: layer } if *layer == value) {
                        removed = true;
                        false
                    } else {
                        true
                    }
                });
                if !removed {
                    let _ = self.states.push(ToggleLayerModifier { value });
                }
                self.handle_terminal_action(coord, context);
            }
            DefaultLayer(value) => {
                self.tap_hold_tracker.coord = coord;
                self.set_default_layer(value);
                self.handle_terminal_action(coord, context);
            }
            Custom(value) => {
                self.tap_hold_tracker.coord = coord;
                self.handle_terminal_action(coord, context);
                if self.states.push(State::Custom { value, coord }).is_ok() {
                    return CustomEvent::Press(value);
                }
            }
        }
        CustomEvent::NoEvent
    }

    fn handle_terminal_action(&mut self, coord: (u8, u8), context: &mut ActionContext) {
        // ignore actions activated by a oneshot
        if !context.inside_oneshot {
            if let Some(oneshot) = &mut self.oneshot {
                oneshot.handle_press(coord, false);
            }
        }
        // ignore actions activated by a tapdance
        if !context.inside_tapdance {
            if let Some(tapdance) = &mut self.tapdance {
                // this will end the current tap dance
                tapdance.handle_press(false);
                self.do_tapdance_action_if_lazy(context);
            }
        }
    }

    /// Obtain the index of the current active layer
    pub fn current_layer(&self) -> usize {
        self.states
            .iter()
            .rev()
            .find_map(State::get_layer)
            .unwrap_or(self.default_layer)
    }

    /// Sets the default layer for the layout
    pub fn set_default_layer(&mut self, value: usize) {
        if value < self.layers.len() {
            self.default_layer = value
        }
    }
}

#[cfg(test)]
mod test {
    extern crate std;
    use super::{Event::*, Layout, *};
    use crate::action::Action::*;
    use crate::action::HoldTapConfig;
    use crate::action::OneShotAction;
    use crate::action::TapDanceAction;
    use crate::action::{k, l, m, t};
    use crate::key_code::KeyCode;
    use crate::key_code::KeyCode::*;
    use std::collections::BTreeSet;

    #[track_caller]
    fn assert_keys(expected: &[KeyCode], iter: impl Iterator<Item = KeyCode>) {
        let expected: BTreeSet<_> = expected.iter().copied().collect();
        let tested = iter.collect();
        assert_eq!(expected, tested);
    }

    #[test]
    fn basic_hold_tap() {
        static mut LAYERS: Layers<2, 1, 2> = [
            [[
                HoldTap(&HoldTapAction {
                    timeout: 200,
                    hold: l(1),
                    tap: k(Space),
                    config: HoldTapConfig::Default,
                    tap_hold_interval: 0,
                }),
                HoldTap(&HoldTapAction {
                    timeout: 200,
                    hold: k(LCtrl),
                    tap: k(Enter),
                    config: HoldTapConfig::Default,
                    tap_hold_interval: 0,
                }),
            ]],
            [[Trans, m(&[LCtrl, Enter].as_slice())]],
        ];
        let mut layout = Layout::new(unsafe { &mut LAYERS });
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Press(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Release(0, 0));
        for _ in 0..197 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[], layout.keycodes());
        }
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LCtrl], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LCtrl], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LCtrl, Space], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LCtrl], layout.keycodes());
        layout.event(Release(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
    }

    #[test]
    fn hold_tap_interleaved_timeout() {
        static mut LAYERS: Layers<2, 1, 1> = [[[
            HoldTap(&HoldTapAction {
                timeout: 200,
                hold: k(LAlt),
                tap: k(Space),
                config: HoldTapConfig::Default,
                tap_hold_interval: 0,
            }),
            HoldTap(&HoldTapAction {
                timeout: 20,
                hold: k(LCtrl),
                tap: k(Enter),
                config: HoldTapConfig::Default,
                tap_hold_interval: 0,
            }),
        ]]];
        let mut layout = Layout::new(unsafe { &mut LAYERS });
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Press(0, 1));
        for _ in 0..15 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[], layout.keycodes());
        }
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[Space], layout.keycodes());
        for _ in 0..10 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[Space], layout.keycodes());
        }
        layout.event(Release(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[Space, LCtrl], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LCtrl], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
    }

    #[test]
    fn hold_on_press() {
        static mut LAYERS: Layers<2, 1, 1> = [[[
            HoldTap(&HoldTapAction {
                timeout: 200,
                hold: k(LAlt),
                tap: k(Space),
                config: HoldTapConfig::HoldOnOtherKeyPress,
                tap_hold_interval: 0,
            }),
            k(Enter),
        ]]];
        let mut layout = Layout::new(unsafe { &mut LAYERS });

        // Press another key before timeout
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Press(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LAlt], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LAlt, Enter], layout.keycodes());
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[Enter], layout.keycodes());
        layout.event(Release(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        // Press another key after timeout
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Press(0, 0));
        for _ in 0..200 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[], layout.keycodes());
        }
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LAlt], layout.keycodes());
        layout.event(Press(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LAlt, Enter], layout.keycodes());
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[Enter], layout.keycodes());
        layout.event(Release(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
    }

    #[test]
    fn permissive_hold() {
        static mut LAYERS: Layers<2, 1, 1> = [[[
            HoldTap(&HoldTapAction {
                timeout: 200,
                hold: k(LAlt),
                tap: k(Space),
                config: HoldTapConfig::PermissiveHold,
                tap_hold_interval: 0,
            }),
            k(Enter),
        ]]];
        let mut layout = Layout::new(unsafe { &mut LAYERS });

        // Press and release another key before timeout
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Press(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Release(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LAlt], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LAlt, Enter], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LAlt], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LAlt], layout.keycodes());
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
    }

    #[test]
    fn multiple_actions() {
        static mut LAYERS: Layers<2, 1, 2> = [
            [[MultipleActions(&[l(1), k(LShift)].as_slice()), k(F)]],
            [[Trans, k(E)]],
        ];
        let mut layout = Layout::new(unsafe { &mut LAYERS });
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift], layout.keycodes());
        layout.event(Press(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift, E], layout.keycodes());
        layout.event(Release(0, 1));
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
    }

    #[test]
    fn custom() {
        static mut LAYERS: Layers<1, 1, 1, u8> = [[[Action::Custom(42)]]];
        let mut layout = Layout::new(unsafe { &mut LAYERS });
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        // Custom event
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::Press(42), layout.tick());
        assert_keys(&[], layout.keycodes());

        // nothing more
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        // release custom
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::Release(42), layout.tick());
        assert_keys(&[], layout.keycodes());
    }

    #[test]
    fn multiple_layers() {
        static mut LAYERS: Layers<2, 1, 4> = [
            [[l(1), l(2)]],
            [[k(A), l(3)]],
            [[l(0), k(B)]],
            [[k(C), k(D)]],
        ];
        let mut layout = Layout::new(unsafe { &mut LAYERS });
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_eq!(0, layout.current_layer());
        assert_keys(&[], layout.keycodes());

        // press L1
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_eq!(1, layout.current_layer());
        assert_keys(&[], layout.keycodes());
        // press L3 on L1
        layout.event(Press(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_eq!(3, layout.current_layer());
        assert_keys(&[], layout.keycodes());
        // release L1, still on l3
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_eq!(3, layout.current_layer());
        assert_keys(&[], layout.keycodes());
        // press and release C on L3
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[C], layout.keycodes());
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        // release L3, back to L0
        layout.event(Release(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_eq!(0, layout.current_layer());
        assert_keys(&[], layout.keycodes());

        // back to empty, going to L2
        layout.event(Press(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_eq!(2, layout.current_layer());
        assert_keys(&[], layout.keycodes());
        // and press the L0 key on L2
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_eq!(0, layout.current_layer());
        assert_keys(&[], layout.keycodes());
        // release the L0, back to L2
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_eq!(2, layout.current_layer());
        assert_keys(&[], layout.keycodes());
        // release the L2, back to L0
        layout.event(Release(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_eq!(0, layout.current_layer());
        assert_keys(&[], layout.keycodes());
    }

    #[test]
    fn custom_handler() {
        fn always_tap(_: StackedIter) -> Option<WaitingAction> {
            Some(WaitingAction::Tap)
        }
        fn always_hold(_: StackedIter) -> Option<WaitingAction> {
            Some(WaitingAction::Hold)
        }
        fn always_nop(_: StackedIter) -> Option<WaitingAction> {
            Some(WaitingAction::NoOp)
        }
        fn always_none(_: StackedIter) -> Option<WaitingAction> {
            None
        }
        static mut LAYERS: Layers<4, 1, 1> = [[[
            HoldTap(&HoldTapAction {
                timeout: 200,
                hold: k(Kb1),
                tap: k(Kb0),
                config: HoldTapConfig::Custom(always_tap),
                tap_hold_interval: 0,
            }),
            HoldTap(&HoldTapAction {
                timeout: 200,
                hold: k(Kb3),
                tap: k(Kb2),
                config: HoldTapConfig::Custom(always_hold),
                tap_hold_interval: 0,
            }),
            HoldTap(&HoldTapAction {
                timeout: 200,
                hold: k(Kb5),
                tap: k(Kb4),
                config: HoldTapConfig::Custom(always_nop),
                tap_hold_interval: 0,
            }),
            HoldTap(&HoldTapAction {
                timeout: 200,
                hold: k(Kb7),
                tap: k(Kb6),
                config: HoldTapConfig::Custom(always_none),
                tap_hold_interval: 0,
            }),
        ]]];
        let mut layout = Layout::new(unsafe { &mut LAYERS });
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        // Custom handler always taps
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[Kb0], layout.keycodes());

        // nothing more
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        // Custom handler always holds
        layout.event(Press(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[Kb3], layout.keycodes());

        // nothing more
        layout.event(Release(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        // Custom handler always prevents any event
        layout.event(Press(0, 2));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        // even timeout does not trigger
        for _ in 0..200 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[], layout.keycodes());
        }

        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        // nothing more
        layout.event(Release(0, 2));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        // Custom handler timeout fallback
        layout.event(Press(0, 3));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        for _ in 0..199 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[], layout.keycodes());
        }

        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[Kb7], layout.keycodes());
    }

    #[test]
    fn tap_hold_interval() {
        static mut LAYERS: Layers<2, 1, 1> = [[[
            HoldTap(&HoldTapAction {
                timeout: 200,
                hold: k(LAlt),
                tap: k(Space),
                config: HoldTapConfig::Default,
                tap_hold_interval: 200,
            }),
            k(Enter),
        ]]];
        let mut layout = Layout::new(unsafe { &mut LAYERS });

        // press and release the HT key, expect tap action
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[Space], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        // press again within tap_hold_interval, tap action should be in keycode immediately
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[Space], layout.keycodes());

        // tap action should continue to be in keycodes even after timeout
        for _ in 0..300 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[Space], layout.keycodes());
        }
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        // Press again. This is outside the tap_hold_interval window, so should result in hold
        // action.
        layout.event(Press(0, 0));
        for _ in 0..200 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[], layout.keycodes());
        }
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LAlt], layout.keycodes());
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
    }

    #[test]
    fn tap_hold_interval_interleave() {
        static mut LAYERS: Layers<3, 1, 1> = [[[
            HoldTap(&HoldTapAction {
                timeout: 200,
                hold: k(LAlt),
                tap: k(Space),
                config: HoldTapConfig::Default,
                tap_hold_interval: 200,
            }),
            k(Enter),
            HoldTap(&HoldTapAction {
                timeout: 200,
                hold: k(LAlt),
                tap: k(Enter),
                config: HoldTapConfig::Default,
                tap_hold_interval: 200,
            }),
        ]]];
        let mut layout = Layout::new(unsafe { &mut LAYERS });

        // press and release the HT key, expect tap action
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[Space], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        // press a different key in between
        layout.event(Press(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[Enter], layout.keycodes());
        layout.event(Release(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        // press HT key again, should result in hold action
        layout.event(Press(0, 0));
        for _ in 0..200 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[], layout.keycodes());
        }
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LAlt], layout.keycodes());
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        // press HT key, press+release diff key, release HT key
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Press(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Release(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[Space], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[Enter, Space], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[Space], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        // press HT key again, should result in hold action
        layout.event(Press(0, 0));
        for _ in 0..200 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[], layout.keycodes());
        }
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LAlt], layout.keycodes());
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        // press HT key, press+release diff (HT) key, release HT key
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Press(0, 2));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Release(0, 2));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[Space], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[Space], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[Enter, Space], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[Space], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        // press HT key again, should result in hold action
        layout.event(Press(0, 0));
        for _ in 0..200 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[], layout.keycodes());
        }
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LAlt], layout.keycodes());
    }

    #[test]
    fn tap_hold_interval_short_hold() {
        static mut LAYERS: Layers<1, 1, 1> = [[[HoldTap(&HoldTapAction {
            timeout: 50,
            hold: k(LAlt),
            tap: k(Space),
            config: HoldTapConfig::Default,
            tap_hold_interval: 200,
        })]]];
        let mut layout = Layout::new(unsafe { &mut LAYERS });

        // press and hold the HT key, expect hold action
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Press(0, 0));
        for _ in 0..50 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[], layout.keycodes());
        }
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LAlt], layout.keycodes());
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        // press and hold the HT key, expect hold action, even though it's within the
        // tap_hold_interval
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Press(0, 0));
        for _ in 0..50 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[], layout.keycodes());
        }
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LAlt], layout.keycodes());
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
    }

    #[test]
    fn tap_hold_interval_different_hold() {
        static mut LAYERS: Layers<2, 1, 1> = [[[
            HoldTap(&HoldTapAction {
                timeout: 50,
                hold: k(LAlt),
                tap: k(Space),
                config: HoldTapConfig::Default,
                tap_hold_interval: 200,
            }),
            HoldTap(&HoldTapAction {
                timeout: 200,
                hold: k(RAlt),
                tap: k(Enter),
                config: HoldTapConfig::Default,
                tap_hold_interval: 200,
            }),
        ]]];
        let mut layout = Layout::new(unsafe { &mut LAYERS });

        // press HT1, press HT2, release HT1 after hold timeout, release HT2, press HT2
        layout.event(Press(0, 0));
        layout.event(Press(0, 1));
        for _ in 0..50 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[], layout.keycodes());
        }
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LAlt], layout.keycodes());
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LAlt], layout.keycodes());
        layout.event(Release(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LAlt, Enter], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[Enter], layout.keycodes());
        // press HT2 again, should result in tap action
        layout.event(Press(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        for _ in 0..300 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[Enter], layout.keycodes());
        }
    }

    #[test]
    fn toggle_multiple_layers() {
        static mut LAYERS: Layers<2, 1, 5> = [
            [[t(1), l(2)]],
            [[k(A), t(1)]],
            [[t(3), k(B)]],
            [[t(3), t(4)]],
            [[t(4), t(3)]],
        ];
        let mut layout = Layout::new(unsafe { &mut LAYERS });
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_eq!(0, layout.current_layer());
        assert_keys(&[], layout.keycodes());

        // toggle L1
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_eq!(1, layout.current_layer());
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_eq!(1, layout.current_layer());
        assert_keys(&[], layout.keycodes());

        // press and release A
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[A], layout.keycodes());
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        // toggle L1 to disable
        layout.event(Press(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_eq!(0, layout.current_layer());
        layout.event(Release(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_eq!(0, layout.current_layer());
        assert_keys(&[], layout.keycodes());

        // press L2
        layout.event(Press(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_eq!(2, layout.current_layer());
        assert_keys(&[], layout.keycodes());

        // toggle L3 on L2
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_eq!(3, layout.current_layer());
        assert_keys(&[], layout.keycodes());

        // release L2, should stay on L3
        layout.event(Release(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_eq!(3, layout.current_layer());
        assert_keys(&[], layout.keycodes());

        // press and release L4 on L3
        layout.event(Press(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_eq!(4, layout.current_layer());
        layout.event(Release(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_eq!(4, layout.current_layer());

        // toggle L3 from L4, should stay on L4
        layout.event(Press(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_eq!(4, layout.current_layer());
        layout.event(Release(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_eq!(4, layout.current_layer());

        // toggle L4 to disable, should be back to L0
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_eq!(0, layout.current_layer());
        assert_keys(&[], layout.keycodes());

        // press L2
        layout.event(Press(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_eq!(2, layout.current_layer());
        assert_keys(&[], layout.keycodes());

        // toggle L3 on L2
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_eq!(3, layout.current_layer());
        assert_keys(&[], layout.keycodes());

        // release L2, should stay on L3
        layout.event(Release(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_eq!(3, layout.current_layer());
        assert_keys(&[], layout.keycodes());

        // press and release L4 on L3
        layout.event(Press(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_eq!(4, layout.current_layer());
        layout.event(Release(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_eq!(4, layout.current_layer());

        // toggle L4 to disable, should be back to L3
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_eq!(3, layout.current_layer());
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_eq!(3, layout.current_layer());

        // toggle L3 to disable, back to L0
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_eq!(0, layout.current_layer());
        assert_keys(&[], layout.keycodes());
    }

    #[test]
    fn one_shot() {
        static mut LAYERS: Layers<3, 1, 1> = [[[
            OneShot(&OneShotAction {
                timeout: 100,
                action: k(LShift),
                end_config: OneShotEndConfig::EndOnFirstPress,
            }),
            k(A),
            k(B),
        ]]];
        let mut layout = Layout::new(unsafe { &mut LAYERS });

        // Test:
        // 1. press one-shot
        // 2. release one-shot
        // 3. press A within timeout
        // 4. press B within timeout
        // 5. release A, B
        layout.event(Press(0, 0));
        for _ in 0..25 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[LShift], layout.keycodes());
        }
        layout.event(Release(0, 0));
        for _ in 0..25 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[LShift], layout.keycodes());
        }
        layout.event(Press(0, 1));
        layout.event(Press(0, 2));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[A, LShift], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[A, B], layout.keycodes());
        layout.event(Release(0, 1));
        layout.event(Release(0, 2));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[B], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        // Test:
        // 1. press one-shot
        // 2. release one-shot
        // 3. press A after timeout
        // 4. release A
        layout.event(Press(0, 0));
        for _ in 0..25 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[LShift], layout.keycodes());
        }
        layout.event(Release(0, 0));
        for _ in 0..75 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[LShift], layout.keycodes());
        }
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Press(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[A], layout.keycodes());
        layout.event(Release(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        // Test:
        // 1. press one-shot
        // 2. press A
        // 3. release A
        // 4. release one-shot
        layout.event(Press(0, 0));
        for _ in 0..25 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[LShift], layout.keycodes());
        }
        layout.event(Press(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift, A], layout.keycodes());
        layout.event(Release(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift], layout.keycodes());
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        // Test:
        // 1. press one-shot
        // 2. press A after timeout
        // 3. release A
        // 4. release one-shot
        layout.event(Press(0, 0));
        for _ in 0..200 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[LShift], layout.keycodes());
        }
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift], layout.keycodes());
        layout.event(Press(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift, A], layout.keycodes());
        layout.event(Release(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift], layout.keycodes());
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
    }

    #[test]
    fn one_shot_overlap_release() {
        static mut LAYERS: Layers<1, 1, 2> = [
            [[OneShot(&OneShotAction {
                timeout: 100,
                action: l(1),
                end_config: OneShotEndConfig::EndOnFirstRelease,
            })]],
            [[k(A)]],
        ];
        let mut layout = Layout::new(unsafe { &mut LAYERS });

        // Test:
        // 1. press one-shot
        // 2. release one-shot
        // 3. press A within timeout
        // 5. release A, should also go back to layer 0
        layout.event(Press(0, 0));
        for _ in 0..25 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_eq!(1, layout.current_layer());
        }
        layout.event(Release(0, 0));
        for _ in 0..25 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_eq!(1, layout.current_layer());
        }
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[A], layout.keycodes());
        assert_eq!(1, layout.current_layer());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[A], layout.keycodes());
        assert_eq!(1, layout.current_layer());
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        assert_eq!(0, layout.current_layer());

        // Test:
        // 1. press one-shot
        // 2. release one-shot
        // 3. press A after timeout
        // 4. release A
        layout.event(Press(0, 0));
        for _ in 0..25 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_eq!(1, layout.current_layer());
        }
        layout.event(Release(0, 0));
        for _ in 0..75 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_eq!(1, layout.current_layer());
        }
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
    }

    #[test]
    fn one_shot_overlap_press() {
        static mut LAYERS: Layers<1, 1, 2> = [
            [[OneShot(&OneShotAction {
                timeout: 100,
                action: l(1),
                end_config: OneShotEndConfig::EndOnFirstPress,
            })]],
            [[k(A)]],
        ];
        let mut layout = Layout::new(unsafe { &mut LAYERS });

        // Test:
        // 1. press one-shot
        // 2. release one-shot
        // 3. press A within timeout
        // 5. release A
        layout.event(Press(0, 0));
        for _ in 0..25 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_eq!(1, layout.current_layer());
        }
        layout.event(Release(0, 0));
        for _ in 0..25 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_eq!(1, layout.current_layer());
        }
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[A], layout.keycodes());
        assert_eq!(1, layout.current_layer());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[A], layout.keycodes());
        assert_eq!(1, layout.current_layer());
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        assert_eq!(0, layout.current_layer());

        // Test:
        // 1. press one-shot
        // 2. release one-shot
        // 3. press A after timeout
        // 4. release A
        layout.event(Press(0, 0));
        for _ in 0..25 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_eq!(1, layout.current_layer());
        }
        layout.event(Release(0, 0));
        for _ in 0..75 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_eq!(1, layout.current_layer());
        }
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
    }

    #[test]
    fn one_shot_end_press_or_repress() {
        static mut LAYERS: Layers<3, 1, 1> = [[[
            OneShot(&OneShotAction {
                timeout: 100,
                action: k(LShift),
                end_config: OneShotEndConfig::EndOnFirstPressOrRepress,
            }),
            k(A),
            k(B),
        ]]];
        let mut layout = Layout::new(unsafe { &mut LAYERS });

        // Test:
        // 1. press one-shot
        // 2. release one-shot
        // 3. press A within timeout
        // 4. press B within timeout
        // 5. release A, B
        layout.event(Press(0, 0));
        for _ in 0..25 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[LShift], layout.keycodes());
        }
        layout.event(Release(0, 0));
        for _ in 0..25 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[LShift], layout.keycodes());
        }
        layout.event(Press(0, 1));
        layout.event(Press(0, 2));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[A, LShift], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[A, B], layout.keycodes());
        layout.event(Release(0, 1));
        layout.event(Release(0, 2));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[B], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        // Test:
        // 1. press one-shot
        // 2. release one-shot
        // 3. press A after timeout
        // 4. release A
        layout.event(Press(0, 0));
        for _ in 0..25 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[LShift], layout.keycodes());
        }
        layout.event(Release(0, 0));
        for _ in 0..75 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[LShift], layout.keycodes());
        }
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Press(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[A], layout.keycodes());
        layout.event(Release(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        // Test:
        // 1. press one-shot
        // 2. press A
        // 3. release A
        // 4. release one-shot
        layout.event(Press(0, 0));
        for _ in 0..25 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[LShift], layout.keycodes());
        }
        layout.event(Press(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift, A], layout.keycodes());
        layout.event(Release(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift], layout.keycodes());
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        // Test:
        // 1. press one-shot
        // 2. press A after timeout
        // 3. release A
        // 4. release one-shot
        layout.event(Press(0, 0));
        for _ in 0..200 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[LShift], layout.keycodes());
        }
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift], layout.keycodes());
        layout.event(Press(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift, A], layout.keycodes());
        layout.event(Release(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift], layout.keycodes());
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        // Test:
        // 1. press one-shot
        // 2. release one-shot
        // 3. press one-shot within timeout
        // 4. release one-shot quickly - should end
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift], layout.keycodes());
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift], layout.keycodes());
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift], layout.keycodes());
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        // Test:
        // 1. press one-shot
        // 2. release one-shot
        // 3. press one-shot within timeout
        // 4. release one-shot after timeout
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift], layout.keycodes());
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift], layout.keycodes());
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift], layout.keycodes());
        for _ in 0..200 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[LShift], layout.keycodes());
        }
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
    }

    #[test]
    fn one_shot_end_on_release() {
        static mut LAYERS: Layers<3, 1, 1> = [[[
            OneShot(&OneShotAction {
                timeout: 100,
                action: k(LShift),
                end_config: OneShotEndConfig::EndOnFirstRelease,
            }),
            k(A),
            k(B),
        ]]];
        let mut layout = Layout::new(unsafe { &mut LAYERS });

        // Test:
        // 1. press one-shot
        // 2. release one-shot
        // 3. press A within timeout
        // 4. press B within timeout
        // 5. release A, B
        layout.event(Press(0, 0));
        for _ in 0..25 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[LShift], layout.keycodes());
        }
        layout.event(Release(0, 0));
        for _ in 0..25 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[LShift], layout.keycodes());
        }
        layout.event(Press(0, 1));
        layout.event(Press(0, 2));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[A, LShift], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[A, B, LShift], layout.keycodes());
        layout.event(Release(0, 1));
        layout.event(Release(0, 2));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[B], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        // Test:
        // 1. press one-shot
        // 2. release one-shot
        // 3. press A after timeout
        // 4. release A
        layout.event(Press(0, 0));
        for _ in 0..25 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[LShift], layout.keycodes());
        }
        layout.event(Release(0, 0));
        for _ in 0..75 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[LShift], layout.keycodes());
        }
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Press(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[A], layout.keycodes());
        layout.event(Release(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        // Test:
        // 1. press one-shot
        // 2. press A
        // 3. release A
        // 4. release one-shot
        layout.event(Press(0, 0));
        for _ in 0..25 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[LShift], layout.keycodes());
        }
        layout.event(Press(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift, A], layout.keycodes());
        layout.event(Release(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift], layout.keycodes());
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        // Test:
        // 1. press one-shot
        // 2. press A after timeout
        // 3. release A
        // 4. release one-shot
        layout.event(Press(0, 0));
        for _ in 0..200 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[LShift], layout.keycodes());
        }
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift], layout.keycodes());
        layout.event(Press(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift, A], layout.keycodes());
        layout.event(Release(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift], layout.keycodes());
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        // Test:
        // 3. press A
        // 1. press one-shot
        // 2. release one-shot
        // 3. release A
        // 4. press B within timeout
        // 5. release B
        layout.event(Press(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[A], layout.keycodes());
        layout.event(Press(0, 0));
        for _ in 0..25 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[A, LShift], layout.keycodes());
        }
        layout.event(Release(0, 0));
        for _ in 0..25 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[A, LShift], layout.keycodes());
        }
        layout.event(Release(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift], layout.keycodes());
        layout.event(Press(0, 2));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[B, LShift], layout.keycodes());
        layout.event(Release(0, 2));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
    }

    #[test]
    fn one_shot_multi() {
        static mut LAYERS: Layers<4, 1, 2> = [
            [[
                OneShot(&OneShotAction {
                    timeout: 100,
                    action: k(LShift),
                    end_config: OneShotEndConfig::EndOnFirstPress,
                }),
                OneShot(&OneShotAction {
                    timeout: 100,
                    action: k(LCtrl),
                    end_config: OneShotEndConfig::EndOnFirstPress,
                }),
                OneShot(&OneShotAction {
                    timeout: 100,
                    action: l(1),
                    end_config: OneShotEndConfig::EndOnFirstPress,
                }),
                NoOp,
            ]],
            [[k(A), k(B), k(C), k(D)]],
        ];
        let mut layout = Layout::new(unsafe { &mut LAYERS });

        layout.event(Press(0, 0));
        layout.event(Release(0, 0));
        for _ in 0..90 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[LShift], layout.keycodes());
        }
        layout.event(Press(0, 1));
        for _ in 0..90 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[LShift, LCtrl], layout.keycodes());
        }
        assert_eq!(layout.current_layer(), 0);
        layout.event(Press(0, 2));
        layout.event(Release(0, 2));
        for _ in 0..90 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[LShift, LCtrl], layout.keycodes());
            assert_eq!(layout.current_layer(), 1);
        }
        layout.event(Press(0, 3));
        layout.event(Release(0, 3));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift, LCtrl, D], layout.keycodes());
        assert_eq!(layout.current_layer(), 1);
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LCtrl], layout.keycodes());
        assert_eq!(layout.current_layer(), 0);
        layout.event(Release(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
    }

    #[test]
    fn one_shot_tap_hold() {
        static mut LAYERS: Layers<3, 1, 2> = [
            [[
                OneShot(&OneShotAction {
                    timeout: 200,
                    action: k(LShift),
                    end_config: OneShotEndConfig::EndOnFirstPress,
                }),
                HoldTap(&HoldTapAction {
                    timeout: 100,
                    hold: k(LAlt),
                    tap: k(Space),
                    config: HoldTapConfig::Default,
                    tap_hold_interval: 0,
                }),
                NoOp,
            ]],
            [[k(A), k(B), k(C)]],
        ];
        let mut layout = Layout::new(unsafe { &mut LAYERS });

        layout.event(Press(0, 0));
        layout.event(Release(0, 0));
        for _ in 0..90 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[LShift], layout.keycodes());
        }
        layout.event(Press(0, 1));
        for _ in 0..90 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[LShift], layout.keycodes());
        }
        layout.event(Release(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift, Space], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        layout.event(Press(0, 0));
        layout.event(Release(0, 0));
        for _ in 0..90 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[LShift], layout.keycodes());
        }
        layout.event(Press(0, 1));
        for _ in 0..100 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[LShift], layout.keycodes());
        }
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift, LAlt], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LAlt], layout.keycodes());
        layout.event(Release(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
    }

    #[test]
    fn tap_dance() {
        static mut LAYERS: Layers<2, 2, 1> = [[
            [
                TapDance(&TapDanceAction {
                    timeout: 100,
                    actions: &[
                        k(LShift),
                        OneShot(&OneShotAction {
                            timeout: 100,
                            action: k(LCtrl),
                            end_config: OneShotEndConfig::EndOnFirstPress,
                        }),
                        HoldTap(&HoldTapAction {
                            timeout: 100,
                            hold: k(LAlt),
                            tap: k(Space),
                            config: HoldTapConfig::Default,
                            tap_hold_interval: 0,
                        }),
                        k(D),
                    ],
                    config: TapDanceConfig::Lazy,
                }),
                k(A),
            ],
            [k(B), k(C)],
        ]];
        let mut layout = Layout::new(unsafe { &mut LAYERS });

        // Test: tap-dance first key, timeout
        layout.event(Press(0, 0));
        for _ in 0..100 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[], layout.keycodes());
        }
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift], layout.keycodes());
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        // Test: tap-dance first key, press another key
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Press(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[A, LShift], layout.keycodes());
        layout.event(Release(0, 0));
        assert_keys(&[A, LShift], layout.keycodes());
        layout.event(Release(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[A], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        // Test: tap-dance second key, timeout
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Release(0, 0));
        for _ in 0..50 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[], layout.keycodes());
        }
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Release(0, 0));
        for _ in 0..99 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[], layout.keycodes());
        }
        for _ in 0..100 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[LCtrl], layout.keycodes());
        }
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        // Test: tap-dance third key, timeout, tap
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Release(0, 0));
        for _ in 0..50 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[], layout.keycodes());
        }
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Release(0, 0));
        for _ in 0..50 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[], layout.keycodes());
        }
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        for _ in 0..98 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[], layout.keycodes());
        }
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[Space], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        // Test: tap-dance third key, timeout, hold
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Release(0, 0));
        for _ in 0..50 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[], layout.keycodes());
        }
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Release(0, 0));
        for _ in 0..50 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[], layout.keycodes());
        }
        layout.event(Press(0, 0));
        for _ in 0..199 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[], layout.keycodes());
        }
        for _ in 0..200 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[LAlt], layout.keycodes());
        }
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        // Test: tap-dance fourth (last key)
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Release(0, 0));
        for _ in 0..50 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[], layout.keycodes());
        }
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Release(0, 0));
        for _ in 0..50 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[], layout.keycodes());
        }
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Release(0, 0));
        for _ in 0..50 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[], layout.keycodes());
        }
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[D], layout.keycodes());
        for _ in 0..200 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[D], layout.keycodes());
        }
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
    }

    #[test]
    fn tap_dance_eager() {
        static mut LAYERS: Layers<2, 2, 1> = [[
            [
                TapDance(&TapDanceAction {
                    timeout: 100,
                    actions: &[k(Kb1), k(Kb2), k(Kb3)],
                    config: TapDanceConfig::Eager,
                }),
                k(A),
            ],
            [k(B), k(C)],
        ]];
        let mut layout = Layout::new(unsafe { &mut LAYERS });

        // Test: tap-dance-eager first key
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[Kb1], layout.keycodes());
        for _ in 0..200 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[Kb1], layout.keycodes());
        }
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        // Test: tap-dance-eager first key, press another key
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[Kb1], layout.keycodes());
        layout.event(Press(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[Kb1, A], layout.keycodes());
        layout.event(Release(0, 0));
        assert_keys(&[Kb1, A], layout.keycodes());
        layout.event(Release(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[A], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        // Test: tap-dance second key
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[Kb1], layout.keycodes());
        layout.event(Release(0, 0));
        for _ in 0..50 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[], layout.keycodes());
        }
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[Kb2], layout.keycodes());
        layout.event(Release(0, 0));
        for _ in 0..99 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[], layout.keycodes());
        }
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        // Test: tap-dance third key
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[Kb1], layout.keycodes());
        layout.event(Release(0, 0));
        for _ in 0..50 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[], layout.keycodes());
        }
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[Kb2], layout.keycodes());
        layout.event(Release(0, 0));
        for _ in 0..50 {
            assert_eq!(CustomEvent::NoEvent, layout.tick());
            assert_keys(&[], layout.keycodes());
        }
        layout.event(Press(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[Kb3], layout.keycodes());
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
    }

    #[test]
    fn sequences() {
        static mut LAYERS: Layers<6, 1, 1> = [[[
            Sequence(
                // Simple Ctrl-C sequence/macro
                &[
                    SequenceEvent::Press(LCtrl),
                    SequenceEvent::Press(C),
                    SequenceEvent::Release(C),
                    SequenceEvent::Release(LCtrl),
                ]
                .as_slice(),
            ),
            Sequence(
                // So we can test that Complete works
                &[
                    SequenceEvent::Press(LCtrl),
                    SequenceEvent::Press(C),
                    SequenceEvent::Complete,
                ]
                .as_slice(),
            ),
            Sequence(
                // YO with a delay in the middle
                &[
                    SequenceEvent::Press(Y),
                    SequenceEvent::Release(Y),
                    // "How many licks does it take to get to the center?"
                    SequenceEvent::Delay(3), // Let's find out
                    SequenceEvent::Press(O),
                    SequenceEvent::Release(O),
                ]
                .as_slice(),
            ),
            Sequence(
                // A long sequence to test the chunking capability
                &[
                    SequenceEvent::Press(LShift), // Important: Shift must remain held
                    SequenceEvent::Press(U),      // ...or the message just isn't the same!
                    SequenceEvent::Release(U),
                    SequenceEvent::Press(N),
                    SequenceEvent::Release(N),
                    SequenceEvent::Press(L),
                    SequenceEvent::Release(L),
                    SequenceEvent::Press(I),
                    SequenceEvent::Release(I),
                    SequenceEvent::Press(M),
                    SequenceEvent::Release(M),
                    SequenceEvent::Press(I),
                    SequenceEvent::Release(I),
                    SequenceEvent::Press(T),
                    SequenceEvent::Release(T),
                    SequenceEvent::Press(E),
                    SequenceEvent::Release(E),
                    SequenceEvent::Press(D),
                    SequenceEvent::Release(D),
                    SequenceEvent::Press(Space),
                    SequenceEvent::Release(Space),
                    SequenceEvent::Press(P),
                    SequenceEvent::Release(P),
                    SequenceEvent::Press(O),
                    SequenceEvent::Release(O),
                    SequenceEvent::Press(W),
                    SequenceEvent::Release(W),
                    SequenceEvent::Press(E),
                    SequenceEvent::Release(E),
                    SequenceEvent::Press(R),
                    SequenceEvent::Release(R),
                    SequenceEvent::Press(Kb1),
                    SequenceEvent::Release(Kb1),
                    SequenceEvent::Press(Kb1),
                    SequenceEvent::Release(Kb1),
                    SequenceEvent::Press(Kb1),
                    SequenceEvent::Release(Kb1),
                    SequenceEvent::Press(Kb1),
                    SequenceEvent::Release(Kb1),
                    SequenceEvent::Release(LShift),
                ]
                .as_slice(),
            ),
            Sequence(
                &[
                    SequenceEvent::Tap(Q),
                    SequenceEvent::Tap(W),
                    SequenceEvent::Tap(E),
                ]
                .as_slice(),
            ),
            Sequence(
                &[
                    SequenceEvent::Tap(X),
                    SequenceEvent::Tap(Y),
                    SequenceEvent::Tap(Z),
                ]
                .as_slice(),
            ),
        ]]];
        let mut layout = Layout::new(unsafe { &mut LAYERS });
        // Test a basic sequence
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Press(0, 0));
        // Sequences take an extra tick to kickoff since the first tick starts the sequence:
        assert_eq!(CustomEvent::NoEvent, layout.tick()); // Sequence detected & added
        assert_eq!(CustomEvent::NoEvent, layout.tick()); // Sequence starts
        assert_keys(&[LCtrl], layout.keycodes()); // First item in the SequenceEvent
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LCtrl, C], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LCtrl], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Release(0, 0));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        // Test the use of Complete()
        assert_keys(&[], layout.keycodes());
        layout.event(Press(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LCtrl], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LCtrl, C], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Release(0, 1));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        // Test a sequence with a Delay() (aka The Mr Owl test; duration == 3)
        layout.event(Press(0, 2));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[Y], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick()); // First decrement (2)
        assert_keys(&[], layout.keycodes()); // "Eh Ooone!"
        assert_eq!(CustomEvent::NoEvent, layout.tick()); // Second decrement (1)
        assert_keys(&[], layout.keycodes()); // "Eh two!"
        assert_eq!(CustomEvent::NoEvent, layout.tick()); // Final decrement (0)
        assert_keys(&[], layout.keycodes()); // "Eh three."
        assert_eq!(CustomEvent::NoEvent, layout.tick()); // Press() added for the next tick()
        assert_eq!(CustomEvent::NoEvent, layout.tick()); // FakeKey Press()
        assert_keys(&[O], layout.keycodes()); // CHOMP!
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Release(0, 2));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        // // Test really long sequences (aka macros)...
        layout.event(Press(0, 3));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift, U], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift, N], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift, L], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift, I], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift, M], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift, I], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift, T], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift, E], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift, D], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift, Space], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift, P], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift, O], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift, W], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift, E], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift, R], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift, Kb1], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift, Kb1], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift, Kb1], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift, Kb1], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[LShift], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        layout.event(Release(0, 3));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        // Test a sequence with Tap Events
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Press(0, 4));
        assert_eq!(CustomEvent::NoEvent, layout.tick()); // Sequence detected & added
        assert_keys(&[], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick()); // To Process Tap(Q)
        assert_keys(&[Q], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick()); // Release(Q)
        assert_keys(&[], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick()); // To Process Tap(W)
        assert_keys(&[W], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick()); // Release(W)
        assert_keys(&[], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick()); // To Process Tap(E)
        assert_keys(&[E], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick()); // Release(E)
        assert_keys(&[], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick()); // Sequence is
                                                         // finished
        assert_keys(&[], layout.keycodes());
        layout.event(Release(0, 4));
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());

        // Test two sequences
        assert_eq!(CustomEvent::NoEvent, layout.tick());
        assert_keys(&[], layout.keycodes());
        layout.event(Press(0, 5));
        assert_eq!(CustomEvent::NoEvent, layout.tick()); // Sequence detected & added
        assert_keys(&[], layout.keycodes());
        layout.event(Press(0, 4));
        assert_eq!(CustomEvent::NoEvent, layout.tick()); // Tap(X)
        assert_keys(&[X], layout.keycodes());
        layout.event(Release(0, 5));
        assert_eq!(CustomEvent::NoEvent, layout.tick()); // Release(X), Tap(Q)
        assert_keys(&[Q], layout.keycodes());
        layout.event(Release(0, 4));
        assert_eq!(CustomEvent::NoEvent, layout.tick()); // Tap(Y), Release(Q)
        assert_keys(&[Y], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick()); // Release(Y), Press(W)
        assert_keys(&[W], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick()); // Press(Z), Release(W)
        assert_keys(&[Z], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick()); // Release(Z), Press(E)
        assert_keys(&[E], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick()); // Release(E)
        assert_keys(&[], layout.keycodes());
        assert_eq!(CustomEvent::NoEvent, layout.tick()); // Sequence is
                                                         // finished
        assert_keys(&[], layout.keycodes());
    }
}
