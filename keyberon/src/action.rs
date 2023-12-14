//! The different actions that can be done.

use crate::key_code::KeyCode;
use crate::layout::{StackedIter, WaitingAction};
use core::fmt::Debug;

/// Behavior configuration of HoldTap.
#[non_exhaustive]
#[derive(Clone, Copy)]
pub enum HoldTapConfig {
    /// Only the timeout will determine between hold and tap action.
    ///
    /// This is a sane default.
    Default,
    /// If there is a key press, the hold action is activated.
    ///
    /// This behavior is interesting for a key which the tap action is
    /// not used in the flow of typing, like escape for example. If
    /// you are annoyed by accidental tap, you can try this behavior.
    HoldOnOtherKeyPress,
    /// If there is a press and release of another key, the hold
    /// action is activated.
    ///
    /// This behavior is interesting for fast typist: the different
    /// between hold and tap would more be based on the sequence of
    /// events than on timing. Be aware that doing the good succession
    /// of key might require some training.
    PermissiveHold,
    /// A custom configuration. Allows the behavior to be controlled by a caller
    /// supplied handler function.
    ///
    /// The input to the custom handler will be an iterator that returns
    /// [Stacked] [Events](Event). The order of the events matches the order the
    /// corresponding key was pressed/released, i.e. the first event is the
    /// event first received after the HoldTap action key is pressed.
    ///
    /// The return value should be the intended action that should be used. A
    /// [Some] value will cause one of: [WaitingAction::Tap] for the configured
    /// tap action, [WaitingAction::Hold] for the hold action, and
    /// [WaitingAction::NoOp] to drop handling of the key press. A [None]
    /// value will cause a fallback to the timeout-based approach. If the
    /// timeout is not triggered, the next tick will call the custom handler
    /// again.
    ///
    /// # Example:
    /// Hold events can be prevented from triggering when pressing multiple keys
    /// on the same side of the keyboard (but does not prevent multiple hold
    /// events).
    /// ```
    /// use keyberon::action::{Action, HoldTapConfig, HoldTapAction};
    /// use keyberon::key_code::KeyCode;
    /// use keyberon::layout::{StackedIter, WaitingAction, Event};
    ///
    /// /// Trigger a `Tap` action on the left side of the keyboard if another
    /// /// key on the left side of the keyboard is pressed.
    /// fn left_mod(stacked_iter: StackedIter) -> Option<WaitingAction> {
    ///     match stacked_iter.map(|s| s.event()).find(|e| e.is_press()) {
    ///         Some(Event::Press(_, j)) if j < 6 => Some(WaitingAction::Tap),
    ///         _ => None,
    ///     }
    /// }
    ///
    /// /// Trigger a `Tap` action on the right side of the keyboard if another
    /// /// key on the right side of the keyboard is pressed.
    /// fn right_mod(stacked_iter: StackedIter) -> Option<WaitingAction> {
    ///     match stacked_iter.map(|s| s.event()).find(|e| e.is_press()) {
    ///         Some(Event::Press(_, j)) if j > 5 => Some(WaitingAction::Tap),
    ///         _ => None,
    ///     }
    /// }
    ///
    /// // Assuming a standard QWERTY layout, the left shift hold action will
    /// // not be triggered when pressing Tab-T, CapsLock-G, nor Shift-B.
    /// const A_SHIFT: Action = Action::HoldTap(&HoldTapAction {
    ///     timeout: 200,
    ///     hold: Action::KeyCode(KeyCode::LShift),
    ///     tap: Action::KeyCode(KeyCode::A),
    ///     config: HoldTapConfig::Custom(left_mod),
    ///     tap_hold_interval: 0,
    /// });
    ///
    /// // Assuming a standard QWERTY layout, the right shift hold action will
    /// // not be triggered when pressing Y-Pipe, H-Enter, nor N-Shift.
    /// const SEMI_SHIFT: Action = Action::HoldTap(&HoldTapAction {
    ///     timeout: 200,
    ///     hold: Action::KeyCode(KeyCode::RShift),
    ///     tap: Action::KeyCode(KeyCode::SColon),
    ///     config: HoldTapConfig::Custom(right_mod),
    ///     tap_hold_interval: 0,
    /// });
    /// ```
    Custom(fn(StackedIter) -> Option<WaitingAction>),
}

impl Debug for HoldTapConfig {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            HoldTapConfig::Default => f.write_str("Default"),
            HoldTapConfig::HoldOnOtherKeyPress => f.write_str("HoldOnOtherKeyPress"),
            HoldTapConfig::PermissiveHold => f.write_str("PermissiveHold"),
            HoldTapConfig::Custom(func) => f
                .debug_tuple("Custom")
                .field(&(*func as fn(StackedIter<'static>) -> Option<WaitingAction>) as &dyn Debug)
                .finish(),
        }
    }
}

impl PartialEq for HoldTapConfig {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (HoldTapConfig::Default, HoldTapConfig::Default)
            | (HoldTapConfig::HoldOnOtherKeyPress, HoldTapConfig::HoldOnOtherKeyPress)
            | (HoldTapConfig::PermissiveHold, HoldTapConfig::PermissiveHold) => true,
            (HoldTapConfig::Custom(self_func), HoldTapConfig::Custom(other_func)) => {
                *self_func as fn(StackedIter<'static>) -> Option<WaitingAction> == *other_func
            }
            _ => false,
        }
    }
}

impl Eq for HoldTapConfig {}

/// Perform different actions on key hold/tap.
///
/// If the key is held more than `timeout` ticks (usually
/// milliseconds), performs the `hold` action, else performs the
/// `tap` action.  Mostly used with a modifier for the hold action
/// and a normal key on the tap action. Any action can be
/// performed, but using a `HoldTap` in a `HoldTap` is not
/// specified (but guaranteed to not crash).
///
/// Different behaviors can be configured using the config field,
/// but whatever the configuration is, if the key is pressed more
/// than `timeout`, the hold action is activated (if no other
/// action was determined before).
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct HoldTapAction<T, K>
where
    T: 'static,
    K: 'static,
{
    /// The duration, in ticks (usually milliseconds) giving the
    /// difference between a hold and a tap.
    pub timeout: u16,
    /// The hold action.
    pub hold: Action<T, K>,
    /// The tap action.
    pub tap: Action<T, K>,
    /// Behavior configuration.
    pub config: HoldTapConfig,
    /// Configuration of the tap and hold holds the tap action.
    ///
    /// If you press and release the key in such a way that the tap
    /// action is performed, and then press it again in less than
    /// `tap_hold_interval` ticks, the tap action will
    /// be held. This allows the tap action to be held by
    /// pressing, releasing and holding the key, allowing the computer
    /// to auto repeat the tap behavior. The timeout starts on the
    /// first press of the key, NOT on the release.
    ///
    /// Pressing a different key in between will not result in the
    /// behaviour described above; the HoldTap key must be pressed twice
    /// in a row.
    ///
    /// To deactivate the functionality, set this to 0.
    pub tap_hold_interval: u16,
}

/// Determine the ending behaviour of the one shot key.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum OneShotEndConfig {
    /// End one shot activation on first non-one-shot key press.
    EndOnFirstPress,
    /// End one shot activation on first non-one-shot key press or a repress of an already-pressed
    /// one-shot key.
    EndOnFirstPressOrRepress,
    /// End one shot activation on first non-one-shot key release.
    EndOnFirstRelease,
    /// End one shot activation on first non-one-shot key release or a repress of an already-pressed
    /// one-shot key.
    EndOnFirstReleaseOrRepress,
}

/// Define one shot key behaviour.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct OneShotAction<T, K>
where
    T: 'static,
    K: 'static,
{
    /// Action to activate until timeout expires or exactly one non-one-shot key is activated.
    pub action: Action<T, K>,
    /// Timeout after which one shot will expire. Note: timeout will be overwritten if another
    /// one shot key is pressed.
    pub timeout: u16,
    /// Configuration of one shot end behaviour. Note: this will be overwritten if another one shot
    /// key is pressed. Consider keeping this consistent between all your one shot keys to prevent
    /// surprising behaviour.
    pub end_config: OneShotEndConfig,
}

/// Determines the behaviour for a [`TapDanceAction`]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum TapDanceConfig {
    /// Eager will activate every action in the sequence as the key is pressed.
    Eager,
    /// Lazy will activate a single action, based on the number of taps, once the tap dance is over
    /// (timeout has been exceeded, or if a different key is pressed). Note that the last action of
    /// the tap dance will still be eagerly activated, and will not require waiting for the
    /// timeout.
    Lazy,
}

/// Settings a [`TapDanceAction`], where an action is executed depending on the number of times the
/// key is pressed.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub struct TapDanceAction<T, K>
where
    T: 'static,
    K: 'static,
{
    /// List of actions that activate based on number of taps. Tapping the tap-dance key once will
    /// activate the action in index 0, three times will activate the action in index 2.
    pub actions: &'static [Action<T, K>],
    /// Timeout after which a tap dance will expire. A new tap for the same tap-dance key will
    /// reset this timeout.
    pub timeout: u16,
    /// Determine behaviour of tap dance.
    pub config: TapDanceConfig,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// The different tyeps of actions we support for key sequences/macros
pub enum SequenceEvent<K>
where
    K: 'static,
{
    /// No operation action: just do nothing (a placeholder).
    NoOp,
    /// A keypress/keydown
    Press(K),
    /// Key release/keyup
    Release(K),
    /// A shortcut for `Press(K), Release(K)`
    Tap(K),
    /// For sequences that need to wait a bit before continuing. The number represents how long (in
    /// ticks) this Delay will last for.
    Delay(u16),
    /// Cancels the running sequence and can be used to mark the end of a sequence.
    Complete,
}

/// The different actions that can be done.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum Action<T = core::convert::Infallible, K = KeyCode>
where
    T: 'static,
    K: 'static,
{
    /// No operation action: just do nothing.
    NoOp,
    /// Transparent, i.e. get the action from the default layer. On
    /// the default layer, it is equivalent to `NoOp`.
    Trans,
    /// A key code, i.e. a classic key.
    KeyCode(K),
    /// Multiple key codes sent at the same time, as if these keys
    /// were pressed at the same time. Useful to send a shifted key,
    /// or complex shortcuts like Ctrl+Alt+Del in a single key press.
    MultipleKeyCodes(&'static &'static [K]),
    /// Multiple actions sent at the same time.
    MultipleActions(&'static &'static [Action<T, K>]),
    /// While pressed, change the current layer. That's the classic
    /// Fn key. If several layer actions are hold at the same time,
    /// the last pressed defines the current layer.
    Layer(usize),
    /// Switch the current layer until the layer gets toggled again.
    /// Make sure to also include a ToggleLayer(x) on layer x, otherwise
    /// you will be stuck on that layer. If multiple layers are toggled on,
    /// the last toggled layer will be the currently active one.
    ToggleLayer(usize),
    /// Change the default layer.
    DefaultLayer(usize),
    /// Perform different actions on key hold/tap (see [`HoldTapAction`]).
    HoldTap(&'static HoldTapAction<T, K>),
    /// One shot key. Also known as "sticky key". See [`OneShotAction`] for configuration info.
    /// Activates `action` until a single other key that is not also a one shot key is used. For
    /// example, a one shot key can be used to activate shift for exactly one keypress or switch to
    /// another layer for exactly one keypress. Holding a one shot key will be treated as a normal
    /// held keypress.
    ///
    /// If you use one shot outside of its intended use cases (modifier key action or layer
    /// action) then you will likely have undesired behaviour. E.g. one shot with the space
    /// key will hold space until either another key is pressed or the timeout occurs, which will
    /// probably send many undesired space characters to your active application.
    OneShot(&'static OneShotAction<T, K>),
    /// Tap-dance key. When tapping the key N times in quick succession, activates the N'th action
    /// in `actions`. The action will activate in the following conditions:
    ///
    /// - a different key is pressed
    /// - `timeout` ticks elapse since the last tap of the same tap-dance key
    /// - the number of taps is equal to the length of `actions`.
    TapDance(&'static TapDanceAction<T, K>),
    /// An array of SequenceEvents that will be triggered (in order)
    Sequence(&'static &'static [SequenceEvent<K>]),
    /// Custom action.
    ///
    /// Define a user defined action. This enum can be anything you
    /// want, as long as it has the `'static` lifetime. It can be used
    /// to drive any non keyboard related actions that you might
    /// manage with key events.
    Custom(T),
}
impl<T, K: Clone> Action<T, K> {
    /// Gets the layer number if the action is the `Layer` action.
    pub fn layer(self) -> Option<usize> {
        match self {
            Action::Layer(l) => Some(l),
            _ => None,
        }
    }
    /// Returns an iterator on the `KeyCode` corresponding to the action.
    pub fn key_codes(&self) -> impl Iterator<Item = K> + '_ {
        match self {
            Action::KeyCode(kc) => core::slice::from_ref(kc).iter().cloned(),
            Action::MultipleKeyCodes(kcs) => kcs.iter().cloned(),
            _ => [].iter().cloned(),
        }
    }
}

/// A shortcut to create a `Action::KeyCode`, useful to create compact
/// layout.
pub const fn k<T, K>(kc: K) -> Action<T, K> {
    Action::KeyCode(kc)
}

/// A shortcut to create a `Action::Layer`, useful to create compact
/// layout.
pub const fn l<T, K>(layer: usize) -> Action<T, K> {
    Action::Layer(layer)
}

/// A shortcut to create a `Action::DefaultLayer`, useful to create compact
/// layout.
pub const fn d<T, K>(layer: usize) -> Action<T, K> {
    Action::DefaultLayer(layer)
}

/// A shortcut to create a `Action::MultipleKeyCodes`, useful to
/// create compact layout.
pub const fn m<T, K>(kcs: &'static &'static [K]) -> Action<T, K> {
    Action::MultipleKeyCodes(kcs)
}

/// A shortcut to create a `Action::ToggleLayer`, useful to create compact layout.
pub const fn t<T, K>(layer: usize) -> Action<T, K> {
    Action::ToggleLayer(layer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::mem;

    #[test]
    fn size_of_action() {
        const PTR_SIZE: usize = mem::size_of::<&()>();
        assert_eq!(mem::size_of::<Action::<(), ()>>(), 2 * PTR_SIZE);
    }
}
