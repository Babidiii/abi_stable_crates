//! Ffi-safe wrapper types around the
//! [flume](https://crates.io/crates/flume/)
//! channel types.

use std::{
    fmt::{self, Debug},
    marker::PhantomData,
};

use flume::{Receiver, RecvError, SendError, Sender, TryRecvError, TrySendError};

use core_extensions::SelfOps;

use crate::{
    marker_type::UnsafeIgnoredType,
    pointer_trait::AsPtr,
    prefix_type::WithMetadata,
    sabi_types::RRef,
    std_types::{RBox, RErr, ROk, ROption, RResult},
    traits::{ErasedType, IntoReprRust},
};

mod errors;
mod extern_fns;
mod iteration;

use self::errors::{RRecvError, RSendError, RTryRecvError, RTrySendError};

pub use self::iteration::{RIntoIter, RIter};

/// Creates a channel which can hold up to `capacity` elements in its internal queue.
///
/// If `capacity==0`,the value must be sent to a receiver in the middle of a `recv` call.
///
/// # Example
///
#[cfg_attr(not(feature = "test_miri_track_raw"), doc = "```rust")]
#[cfg_attr(feature = "test_miri_track_raw", doc = "```ignore")]
/// use abi_stable::external_types::flume as mpmc;
///
/// let (tx, rx) = mpmc::bounded::<u32>(3);
///
/// std::thread::spawn(move || {
///     tx.send(10).unwrap();
///     tx.send(11).unwrap();
///     tx.send(12).unwrap();
/// });
///
/// assert_eq!(rx.recv().unwrap(), 10);
/// assert_eq!(rx.recv().unwrap(), 11);
/// assert_eq!(rx.recv().unwrap(), 12);
/// assert!(rx.try_recv().is_err());
///
/// ```
///
pub fn bounded<T>(capacity: usize) -> (RSender<T>, RReceiver<T>) {
    let (tx, rx) = flume::bounded::<T>(capacity);
    (tx.into(), rx.into())
}

/// Creates a channel which can hold an unbounded amount elements in its internal queue.
///
/// # Example
///
#[cfg_attr(not(feature = "test_miri_track_raw"), doc = "```rust")]
#[cfg_attr(feature = "test_miri_track_raw", doc = "```ignore")]
/// use abi_stable::external_types::flume as mpmc;
///
/// let (tx, rx) = mpmc::unbounded::<&'static str>();
///
/// let join_guard = std::thread::spawn(move || {
///     assert_eq!(rx.recv().unwrap(), "foo");
///     assert_eq!(rx.recv().unwrap(), "bar");
///     assert_eq!(rx.recv().unwrap(), "baz");
///     assert!(rx.try_recv().is_err());
/// });
///
/// tx.send("foo").unwrap();
/// tx.send("bar").unwrap();
/// tx.send("baz").unwrap();
///
/// join_guard.join().unwrap();
///
/// ```
///
///
pub fn unbounded<T>() -> (RSender<T>, RReceiver<T>) {
    let (tx, rx) = flume::unbounded::<T>();
    (tx.into(), rx.into())
}

///////////////////////////////////////////////////////////////////////////////

/// The sender end of a channel,
/// which can be either bounded or unbounded.
#[repr(C)]
#[derive(StableAbi)]
pub struct RSender<T> {
    channel: RBox<ErasedSender<T>>,
    vtable: VTable_Ref<T>,
}

impl<T> RSender<T> {
    fn vtable(&self) -> VTable_Ref<T> {
        self.vtable
    }

    /// Blocks until `value` is either sent,or the the other end is disconnected.
    ///
    /// If the channel queue is full,this will block to send `value`.
    ///
    /// If the channel is disconnected,this will return an error with `value`.
    pub fn send(&self, value: T) -> Result<(), SendError<T>> {
        let vtable = self.vtable();

        vtable.send()(self.channel.as_rref(), value).piped(result_from)
    }

    /// Immediately sends `value`,or returns with an error.
    ///
    /// An error will be returned in these 2 conditions:
    ///
    /// - the channel is full.
    ///
    /// - the channel has been disconnected.
    ///
    /// If the channel has a capacity of 0,it will only send `value` if
    /// the other end is calling `recv`.
    ///
    /// # Example
    ///
    #[cfg_attr(not(feature = "test_miri_track_raw"), doc = "```rust")]
    #[cfg_attr(feature = "test_miri_track_raw", doc = "```ignore")]
    /// use abi_stable::external_types::flume as mpmc;
    ///
    /// let (tx, rx) = mpmc::bounded::<bool>(1);
    ///
    /// tx.try_send(true).unwrap();
    /// assert!(tx.try_send(true).unwrap_err().is_full());
    ///
    /// drop(rx);
    /// assert!(tx.try_send(false).unwrap_err().is_disconnected());
    ///
    /// ```
    ///
    ///
    pub fn try_send(&self, value: T) -> Result<(), TrySendError<T>> {
        let vtable = self.vtable();

        vtable.try_send()(self.channel.as_rref(), value).piped(result_from)
    }

    /// Returns true if there are no values in the channel queue.
    ///
    /// # Example
    ///
    #[cfg_attr(not(feature = "test_miri_track_raw"), doc = "```rust")]
    #[cfg_attr(feature = "test_miri_track_raw", doc = "```ignore")]
    /// use abi_stable::external_types::flume as mpmc;
    ///
    /// let (tx, rx) = mpmc::bounded::<()>(1);
    ///
    /// assert!(tx.is_empty());
    ///
    /// tx.send(()).unwrap();
    /// assert!(!tx.is_empty());
    ///
    /// rx.recv().unwrap();
    /// assert!(tx.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        let vtable = self.vtable();

        vtable.sender_is_empty()(self.channel.as_rref())
    }

    /// Returns true if the channel queue is full.
    ///
    /// This always returns true for channels constructed with `bounded(0)`.
    ///
    /// # Example
    ///
    #[cfg_attr(not(feature = "test_miri_track_raw"), doc = "```rust")]
    #[cfg_attr(feature = "test_miri_track_raw", doc = "```ignore")]
    /// use abi_stable::external_types::flume as mpmc;
    ///
    /// let (tx, rx) = mpmc::bounded::<()>(2);
    ///
    /// assert!(!tx.is_full());
    ///
    /// tx.send(()).unwrap();
    /// assert!(!tx.is_full());
    ///
    /// tx.send(()).unwrap();
    /// assert!(tx.is_full());
    ///
    /// rx.recv().unwrap();
    /// assert!(!tx.is_full());
    /// ```
    pub fn is_full(&self) -> bool {
        let vtable = self.vtable();

        vtable.sender_is_full()(self.channel.as_rref())
    }

    /// Returns the amount of values in the channel queue.
    ///
    /// # Example
    ///
    #[cfg_attr(not(feature = "test_miri_track_raw"), doc = "```rust")]
    #[cfg_attr(feature = "test_miri_track_raw", doc = "```ignore")]
    /// use abi_stable::external_types::flume as mpmc;
    ///
    /// let (tx, rx) = mpmc::bounded::<()>(2);
    ///
    /// assert_eq!(tx.len(), 0);
    ///
    /// tx.send(()).unwrap();
    /// assert_eq!(tx.len(), 1);
    ///
    /// tx.send(()).unwrap();
    /// assert_eq!(tx.len(), 2);
    ///
    /// rx.recv().unwrap();
    /// assert_eq!(tx.len(), 1);
    ///
    /// ```
    pub fn len(&self) -> usize {
        let vtable = self.vtable();

        vtable.sender_len()(self.channel.as_rref())
    }

    /// Returns the amount of values the channel queue can hold.
    ///
    /// This returns None if the channel is unbounded.
    ///
    /// # Example
    ///
    #[cfg_attr(not(feature = "test_miri_track_raw"), doc = "```rust")]
    #[cfg_attr(feature = "test_miri_track_raw", doc = "```ignore")]
    /// use abi_stable::external_types::flume as mpmc;
    ///
    /// {
    ///     let (tx, rx) = mpmc::bounded::<()>(2);
    ///     assert_eq!(tx.capacity(), Some(2));
    /// }
    /// {
    ///     let (tx, rx) = mpmc::unbounded::<()>();
    ///     assert_eq!(tx.capacity(), None);
    /// }
    ///
    /// ```
    pub fn capacity(&self) -> Option<usize> {
        let vtable = self.vtable();

        vtable.sender_capacity()(self.channel.as_rref()).into_rust()
    }

    ///
    pub fn sender_count(&self) -> usize {
        let vtable = self.vtable();

        vtable.sender_sender_count()(self.channel.as_rref())
    }

    ///
    pub fn receiver_count(&self) -> usize {
        let vtable = self.vtable();

        vtable.sender_receiver_count()(self.channel.as_rref())
    }
}

impl<T> Clone for RSender<T> {
    /// Clones this channel end,getting another handle into the channel.
    ///
    /// Note that this allocates an RBox<_>.
    fn clone(&self) -> Self {
        let vtable = self.vtable();

        Self {
            channel: vtable.clone_sender()(self.channel.as_rref()),
            vtable: self.vtable,
        }
    }
}

impl<T> Debug for RSender<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt("RSender{..}", f)
    }
}

unsafe impl<T: Send> Sync for RSender<T> {}

unsafe impl<T: Send> Send for RSender<T> {}

impl_from_rust_repr! {
    impl[T] From<Sender<T>> for RSender<T> {
        fn(this){
            Self{
                channel:ErasedSender::from_unerased_value(this),
                vtable: MakeVTable::<T>::VTABLE,
            }
        }
    }
}

///////////////////////////////////////////////////////////////////////////////

/// The receiver end of a channel,
/// which can be either bounded or unbounded.
///
/// # Examples
///
#[cfg_attr(not(feature = "test_miri_track_raw"), doc = "```rust")]
#[cfg_attr(feature = "test_miri_track_raw", doc = "```ignore")]
/// use abi_stable::external_types::flume as mpmc;
///
/// let (tx, rx) = mpmc::unbounded::<&'static str>();
///
/// let join_guard = std::thread::spawn(move || {
///     assert_eq!(rx.recv().unwrap(), "PING");
///     assert_eq!(rx.recv().unwrap(), "PING");
///     assert_eq!(rx.recv().unwrap(), "PING");
///     assert_eq!(rx.recv().unwrap(), "PING");
///     assert!(rx.try_recv().unwrap_err().is_empty());
/// });
///
/// for _ in 0..4 {
///     tx.send("PING").unwrap();
/// }
///
/// join_guard.join().unwrap();
///
/// assert!(tx.send("").is_err());
///
/// ```
///
#[repr(C)]
#[derive(StableAbi)]
pub struct RReceiver<T> {
    channel: RBox<ErasedReceiver<T>>,
    vtable: VTable_Ref<T>,
}

impl<T> RReceiver<T> {
    fn vtable(&self) -> VTable_Ref<T> {
        self.vtable
    }

    /// Blocks until a value is either received,or the the other end is disconnected.
    ///
    /// If the channel queue is empty,this will block to receive a value.
    ///
    /// This will return an error if the channel is disconnected.
    ///
    ///
    /// # Example
    ///
    #[cfg_attr(not(feature = "test_miri_track_raw"), doc = "```rust")]
    #[cfg_attr(feature = "test_miri_track_raw", doc = "```ignore")]
    /// use abi_stable::external_types::flume as mpmc;
    ///
    /// let (tx, rx) = mpmc::bounded::<&'static str>(3);
    ///
    /// tx.send("J__e H____y").unwrap();
    /// assert_eq!(rx.recv().unwrap(), "J__e H____y");
    ///
    /// drop(tx);
    /// assert!(rx.recv().is_err());
    ///
    /// ```
    ///
    pub fn recv(&self) -> Result<T, RecvError> {
        let vtable = self.vtable();

        vtable.recv()(self.channel.as_rref()).piped(result_from)
    }

    /// Immediately receives a value,or returns with an error.
    ///
    /// An error will be returned in these 2 conditions:
    ///
    /// - the channel is empty.
    ///
    /// - the channel has been disconnected.
    ///
    /// If the channel has a capacity of 0,it will only receive a value if
    /// the other end is calling `send`.
    ///
    ///
    /// # Example
    ///
    #[cfg_attr(not(feature = "test_miri_track_raw"), doc = "```rust")]
    #[cfg_attr(feature = "test_miri_track_raw", doc = "```ignore")]
    /// use abi_stable::external_types::flume as mpmc;
    ///
    /// let (tx, rx) = mpmc::bounded::<&'static str>(3);
    ///
    /// assert!(rx.try_recv().is_err());
    ///
    /// tx.send("D__e S_____r").unwrap();
    /// assert_eq!(rx.try_recv().unwrap(), "D__e S_____r");
    ///
    /// drop(tx);
    /// assert!(rx.try_recv().is_err());
    ///
    /// ```
    pub fn try_recv(&self) -> Result<T, TryRecvError> {
        let vtable = self.vtable();

        vtable.try_recv()(self.channel.as_rref()).piped(result_from)
    }

    /// Returns true if there are no values in the channel queue.
    ///
    /// # Example
    ///
    #[cfg_attr(not(feature = "test_miri_track_raw"), doc = "```rust")]
    #[cfg_attr(feature = "test_miri_track_raw", doc = "```ignore")]
    /// use abi_stable::external_types::flume as mpmc;
    ///
    /// let (tx, rx) = mpmc::bounded::<()>(1);
    ///
    /// assert!(rx.is_empty());
    ///
    /// tx.send(()).unwrap();
    /// assert!(!rx.is_empty());
    ///
    /// rx.recv().unwrap();
    /// assert!(rx.is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        let vtable = self.vtable();

        vtable.receiver_is_empty()(self.channel.as_rref())
    }

    /// Returns true if the channel queue is full.
    ///
    /// This always returns true for channels constructed with `bounded(0)`.
    ///
    /// # Example
    ///
    #[cfg_attr(not(feature = "test_miri_track_raw"), doc = "```rust")]
    #[cfg_attr(feature = "test_miri_track_raw", doc = "```ignore")]
    /// use abi_stable::external_types::flume as mpmc;
    ///
    /// let (tx, rx) = mpmc::bounded::<()>(2);
    ///
    /// assert!(!rx.is_full());
    ///
    /// tx.send(()).unwrap();
    /// assert!(!rx.is_full());
    ///
    /// tx.send(()).unwrap();
    /// assert!(rx.is_full());
    ///
    /// rx.recv().unwrap();
    /// assert!(!rx.is_full());
    /// ```
    pub fn is_full(&self) -> bool {
        let vtable = self.vtable();

        vtable.receiver_is_full()(self.channel.as_rref())
    }

    /// Returns the amount of values in the channel queue.
    ///
    /// # Example
    ///
    #[cfg_attr(not(feature = "test_miri_track_raw"), doc = "```rust")]
    #[cfg_attr(feature = "test_miri_track_raw", doc = "```ignore")]
    /// use abi_stable::external_types::flume as mpmc;
    ///
    /// let (tx, rx) = mpmc::bounded::<()>(2);
    ///
    /// assert_eq!(rx.len(), 0);
    ///
    /// tx.send(()).unwrap();
    /// assert_eq!(rx.len(), 1);
    ///
    /// tx.send(()).unwrap();
    /// assert_eq!(rx.len(), 2);
    ///
    /// rx.recv().unwrap();
    /// assert_eq!(rx.len(), 1);
    ///
    /// ```
    pub fn len(&self) -> usize {
        let vtable = self.vtable();

        vtable.receiver_len()(self.channel.as_rref())
    }

    /// Returns the amount of values the channel queue can hold.
    ///
    /// This returns None if the channel is unbounded.
    ///
    /// # Example
    ///
    #[cfg_attr(not(feature = "test_miri_track_raw"), doc = "```rust")]
    #[cfg_attr(feature = "test_miri_track_raw", doc = "```ignore")]
    /// use abi_stable::external_types::flume as mpmc;
    ///
    /// {
    ///     let (tx, rx) = mpmc::bounded::<()>(2);
    ///     assert_eq!(rx.capacity(), Some(2));
    /// }
    /// {
    ///     let (tx, rx) = mpmc::unbounded::<()>();
    ///     assert_eq!(rx.capacity(), None);
    /// }
    ///
    /// ```
    pub fn capacity(&self) -> Option<usize> {
        let vtable = self.vtable();

        vtable.receiver_capacity()(self.channel.as_rref()).into_rust()
    }

    /// Creates an Iterator that receives values from the channel.
    ///
    /// # Example
    ///
    #[cfg_attr(not(feature = "test_miri_track_raw"), doc = "```rust")]
    #[cfg_attr(feature = "test_miri_track_raw", doc = "```ignore")]
    /// use abi_stable::external_types::flume as mpmc;
    ///
    /// use std::thread;
    ///
    /// let (tx, rx) = mpmc::bounded::<usize>(1);
    ///
    /// thread::spawn(move || {
    ///     for i in 0..1000 {
    ///         tx.send(i).unwrap();
    ///     }
    /// });
    ///
    /// for (i, n) in rx.iter().enumerate() {
    ///     assert_eq!(i, n);
    /// }
    ///
    /// ```
    pub fn iter(&self) -> RIter<'_, T> {
        RIter { channel: self }
    }

    ///
    pub fn sender_count(&self) -> usize {
        let vtable = self.vtable();

        vtable.receiver_sender_count()(self.channel.as_rref())
    }

    ///
    pub fn receiver_count(&self) -> usize {
        let vtable = self.vtable();

        vtable.receiver_receiver_count()(self.channel.as_rref())
    }
}

impl<'a, T> IntoIterator for &'a RReceiver<T> {
    type Item = T;
    type IntoIter = RIter<'a, T>;

    /// Creates an Iterator that receives values from the channel.
    #[inline]
    fn into_iter(self) -> RIter<'a, T> {
        self.iter()
    }
}

impl<T> IntoIterator for RReceiver<T> {
    type Item = T;
    type IntoIter = RIntoIter<T>;

    /// Creates an Iterator that receives values from the channel.
    #[inline]
    fn into_iter(self) -> RIntoIter<T> {
        RIntoIter { channel: self }
    }
}

impl<T> Clone for RReceiver<T> {
    /// Clones this channel end,getting another handle into the channel.
    ///
    /// Note that this allocates an RBox<_>.
    fn clone(&self) -> Self {
        let vtable = self.vtable();

        Self {
            channel: vtable.clone_receiver()(self.channel.as_rref()),
            vtable: self.vtable,
        }
    }
}

impl<T> Debug for RReceiver<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        Debug::fmt("RReceiver{..}", f)
    }
}

unsafe impl<T: Send> Sync for RReceiver<T> {}

unsafe impl<T: Send> Send for RReceiver<T> {}

impl_from_rust_repr! {
    impl[T] From<Receiver<T>> for RReceiver<T> {
        fn(this){
            Self{
                channel:ErasedReceiver::from_unerased_value(this),
                vtable:MakeVTable::<T>::VTABLE,
            }
        }
    }
}

///////////////////////////////////////////////////////////////////////////////

#[inline]
fn result_from<T, E0, E1>(res: RResult<T, E0>) -> Result<T, E1>
where
    E0: Into<E1>,
{
    match res {
        ROk(x) => Ok(x),
        RErr(e) => Err(e.into()),
    }
}

#[repr(C)]
#[derive(StableAbi)]
struct ErasedSender<T>(PhantomData<T>, UnsafeIgnoredType<Sender<T>>);

impl<T> ErasedType<'_> for ErasedSender<T> {
    type Unerased = Sender<T>;
}

#[repr(C)]
#[derive(StableAbi)]
struct ErasedReceiver<T>(PhantomData<T>, UnsafeIgnoredType<Receiver<T>>);

impl<T> ErasedType<'_> for ErasedReceiver<T> {
    type Unerased = Receiver<T>;
}

///////////////////////////////////////////////////////////////////////////////

#[repr(C)]
#[derive(StableAbi)]
#[sabi(kind(Prefix))]
#[sabi(missing_field(panic))]
//#[sabi(debug_print)]
struct VTable<T> {
    send: extern "C" fn(this: RRef<'_, ErasedSender<T>>, T) -> RResult<(), RSendError<T>>,
    try_send: extern "C" fn(this: RRef<'_, ErasedSender<T>>, T) -> RResult<(), RTrySendError<T>>,
    clone_sender: extern "C" fn(this: RRef<'_, ErasedSender<T>>) -> RBox<ErasedSender<T>>,
    sender_is_empty: extern "C" fn(this: RRef<'_, ErasedSender<T>>) -> bool,
    sender_is_full: extern "C" fn(this: RRef<'_, ErasedSender<T>>) -> bool,
    sender_len: extern "C" fn(this: RRef<'_, ErasedSender<T>>) -> usize,
    sender_capacity: extern "C" fn(this: RRef<'_, ErasedSender<T>>) -> ROption<usize>,
    sender_sender_count: extern "C" fn(this: RRef<'_, ErasedSender<T>>) -> usize,
    sender_receiver_count: extern "C" fn(this: RRef<'_, ErasedSender<T>>) -> usize,

    recv: extern "C" fn(this: RRef<'_, ErasedReceiver<T>>) -> RResult<T, RRecvError>,
    try_recv: extern "C" fn(this: RRef<'_, ErasedReceiver<T>>) -> RResult<T, RTryRecvError>,
    clone_receiver: extern "C" fn(this: RRef<'_, ErasedReceiver<T>>) -> RBox<ErasedReceiver<T>>,
    receiver_is_empty: extern "C" fn(this: RRef<'_, ErasedReceiver<T>>) -> bool,
    receiver_is_full: extern "C" fn(this: RRef<'_, ErasedReceiver<T>>) -> bool,
    receiver_len: extern "C" fn(this: RRef<'_, ErasedReceiver<T>>) -> usize,
    receiver_capacity: extern "C" fn(this: RRef<'_, ErasedReceiver<T>>) -> ROption<usize>,
    receiver_sender_count: extern "C" fn(this: RRef<'_, ErasedReceiver<T>>) -> usize,
    #[sabi(last_prefix_field)]
    receiver_receiver_count: extern "C" fn(this: RRef<'_, ErasedReceiver<T>>) -> usize,
}

struct MakeVTable<'a, T>(&'a T);

impl<'a, T: 'a> MakeVTable<'a, T> {
    const VALUE: VTable<T> = VTable {
        send: ErasedSender::send,
        try_send: ErasedSender::try_send,
        clone_sender: ErasedSender::clone,
        sender_is_empty: ErasedSender::is_empty,
        sender_is_full: ErasedSender::is_full,
        sender_len: ErasedSender::len,
        sender_capacity: ErasedSender::capacity,
        sender_sender_count: ErasedSender::sender_count,
        sender_receiver_count: ErasedSender::receiver_count,

        recv: ErasedReceiver::recv,
        try_recv: ErasedReceiver::try_recv,
        clone_receiver: ErasedReceiver::clone,
        receiver_is_empty: ErasedReceiver::is_empty,
        receiver_is_full: ErasedReceiver::is_full,
        receiver_len: ErasedReceiver::len,
        receiver_capacity: ErasedReceiver::capacity,
        receiver_sender_count: ErasedReceiver::sender_count,
        receiver_receiver_count: ErasedReceiver::receiver_count,
    };

    staticref! {
        const WM_VALUE: WithMetadata<VTable<T>> = WithMetadata::new(Self::VALUE)
    }

    // The VTABLE for this type in this executable/library
    const VTABLE: VTable_Ref<T> = VTable_Ref(Self::WM_VALUE.as_prefix());
}
