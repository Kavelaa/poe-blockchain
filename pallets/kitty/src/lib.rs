#![cfg_attr(not(feature = "std"), no_std)]

/// Edit this file to define custom logic or remove it if it is not needed.
/// Learn more about FRAME and the core library of Substrate FRAME pallets:
/// <https://substrate.dev/docs/en/knowledgebase/runtime/frame>
pub use pallet::*;

// #[cfg(test)]
// mod mock;

// #[cfg(test)]
// mod tests;

#[frame_support::pallet]
pub mod pallet {
    use codec::{Decode, Encode};
    use frame_support::{
        dispatch::DispatchResult,
        pallet_prelude::*,
        sp_io::hashing::blake2_128,
        traits::{Currency, ExistenceRequirement, Randomness},
    };
    use frame_system::pallet_prelude::*;

    #[derive(Encode, Decode)]
    pub struct Kitty(pub [u8; 16]);

    /// Configure the pallet by specifying the parameters and types on which it depends.
    #[pallet::config]
    pub trait Config: frame_system::Config {
        /// Because this pallet emits events, it depends on the runtime's definition of an event.
        type Event: From<Event<Self>> + IsType<<Self as frame_system::Config>::Event>;
        type Randomness: Randomness<Self::Hash, Self::BlockNumber>;
        type KittyIndex: u32;
    }

    #[pallet::pallet]
    #[pallet::generate_store(pub(super) trait Store)]
    pub struct Pallet<T>(_);

    // Pallets use events to inform users when important changes are made.
    // https://substrate.dev/docs/en/knowledgebase/runtime/events
    #[pallet::event]
    #[pallet::metadata(T::AccountId = "AccountId")]
    #[pallet::generate_deposit(pub(super) fn deposit_event)]
    pub enum Event<T: Config> {
        KittyCreate(T::AccountId, KittyIndex),
        KittyTransfer(T::AccountId, T::AccountId, KittyIndex),
        KittyPriceSet(KittyIndex, u32),
        KittyBought(T::AccountId, T::AccountId, KittyIndex, u32),
    }

    // Errors inform users that something went wrong.
    #[pallet::error]
    pub enum Error<T> {
        /// Kitties count overflow
        KittiesCountOverflow,
        NotOwner,
        SameParentIndex,
        InvalidKittyIndex,
    }

    // The pallet's runtime storage items.
    // https://substrate.dev/docs/en/knowledgebase/runtime/storage
    #[pallet::storage]
    #[pallet::getter(fn kitties_count)]
    // Learn more about declaring storage items:
    // https://substrate.dev/docs/en/knowledgebase/runtime/storage#declaring-storage-items
    pub type KittiesCount<T> = StorageValue<_, u32>;

    #[pallet::storage]
    #[pallet::getter(fn kitties)]
    pub type Kitties<T> = StorageMap<_, Blake2_128Concat, KittyIndex, Option<Kitty>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn kittyies_price)]
    pub type KittiesPrice<T> = StorageMap<_, Blake2_128Concat, KittyIndex, Option<u32>, ValueQuery>;

    #[pallet::storage]
    #[pallet::getter(fn owner)]
    pub type Owner<T: Config> =
        StorageMap<_, Blake2_128Concat, KittyIndex, Option<T::AccountId>, ValueQuery>;

    #[pallet::hooks]
    impl<T: Config> Hooks<BlockNumberFor<T>> for Pallet<T> {}

    // Dispatchable functions allows users to interact with the pallet and invoke state changes.
    // These functions materialize as "extrinsics", which are often compared to transactions.
    // Dispatchable functions must be annotated with a weight and must return a DispatchResult.
    #[pallet::call]
    impl<T: Config> Pallet<T> {
        #[pallet::weight(0)]
        pub fn create(origin: OriginFor<T>) -> DispatchResult {
            // Check that the extrinsic was signed and get the signer.
            // This function will return an error if the extrinsic is not signed.
            // https://substrate.dev/docs/en/knowledgebase/runtime/origin
            let who = ensure_signed(origin)?;

            let kitty_id = match Self::kitties_count() {
                Some(id) => {
                    ensure!(
                        id != KittyIndex::max_value(),
                        Error::<T>::KittiesCountOverflow
                    );
                    id
                }
                None => 1,
            };

            let dna = Self::random_value(&who);

            Self::mint_kitty(who, kitty_id, dna);

            Ok(())
        }

        #[pallet::weight(0)]
        pub fn transfer(
            origin: OriginFor<T>,
            new_owner: T::AccountId,
            kitty_id: KittyIndex,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;

            ensure!(
                Some(who.clone()) == Owner::<T>::get(kitty_id),
                Error::<T>::NotOwner
            );

            Owner::<T>::insert(kitty_id, Some(new_owner.clone()));

            Self::deposit_event(Event::KittyTransfer(who, new_owner, kitty_id));

            Ok(())
        }

        #[pallet::weight(0)]
        pub fn breed(
            origin: OriginFor<T>,
            kitty_id_1: KittyIndex,
            kitty_id_2: KittyIndex,
        ) -> DispatchResult {
            let who = ensure_signed(origin)?;
            ensure!(kitty_id_1 != kitty_id_2, Error::<T>::SameParentIndex);

            let kitty1 = Self::kitties(kitty_id_1).ok_or(Error::<T>::InvalidKittyIndex)?;
            let kitty2 = Self::kitties(kitty_id_2).ok_or(Error::<T>::InvalidKittyIndex)?;

            let kitty_id = match Self::kitties_count() {
                Some(id) => {
                    ensure!(
                        id != KittyIndex::max_value(),
                        Error::<T>::KittiesCountOverflow
                    );
                    id
                }
                None => 1,
            };

            let dna_1 = kitty1.0;
            let dna_2 = kitty2.0;

            let selector = Self::random_value(&who);
            let mut new_dna = [0u8; 16];

            for i in 0..dna_1.len() {
                new_dna[i] = (selector[i] & dna_1[i]) | (!selector[i] & dna_2[i]);
            }

            Self::mint_kitty(who, kitty_id, new_dna);

            Ok(())
        }

        #[pallet::weight(0)]
        pub fn set_kitty_price(
            origin: OriginFor<T>,
            kitty_id: KittyIndex,
            price: u32,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;

            // Kitty存在
            ensure!(
                <Kitties<T>>::contains_key(kitty_id),
                "This cat does not exist"
            );

            // 是kitty所有者在调用extrinsic
            ensure!(
                Owner::<T>::get(kitty_id) == Some(who),
                "You do not own this cat"
            );

            // 给kitty设置价格
            KittiesPrice::<T>::insert(kitty_id, Some(price));

            // Deposit a "PriceSet" event.
            Self::deposit_event(Event::KittyPriceSet(kitty_id, price));

            Ok(())
        }

        #[pallet::weight(0)]
        pub fn buy_kitty(
            origin: OriginFor<T>,
            kitty_id: KittyIndex,
            ask_price: u32,
        ) -> DispatchResultWithPostInfo {
            let who = ensure_signed(origin)?;

            ensure!(
                <Kitties<T>>::contains_key(kitty_id),
                "This can does not exist"
            );

            let owner = Owner::<T>::get(kitty_id);

            ensure!(owner != Some(who), "You can't buy your own cat");

            ensure!(
                KittiesPrice::<T>::contains_key(kitty_id),
                "This kitty is not for sale"
            );

            let kitty_price = KittiesPrice::<T>::get(kitty_id)?;

            ensure!(
                kitty_price <= ask_price,
                "This Kitty is out of your budget!"
            );

            // 支付
            <pallet_balances::Pallet<T> as Currency<_>>::transfer(
                &who,
                &owner,
                kitty_price,
                ExistenceRequirement::KeepAlive,
            )?;

            // 转移所有权
            Owner::<T>::insert(kitty_id, Some(who));

            // 取消售卖
            KittiesPrice::<T>::remove(kitty_id);

            Self::deposit_event(Event::KittyBought(who, owner, kitty_id, ask_price));

            Ok(())
        }
    }

    impl<T: Config> Pallet<T> {
        fn random_value(sender: &T::AccountId) -> [u8; 16] {
            let payload = (
                T::Randomness::random_seed(),
                &sender,
                <frame_system::Pallet<T>>::extrinsic_index(),
            );
            payload.using_encoded(blake2_128)
        }

        fn mint_kitty(owner: T::AccountId, kitty_id: KittyIndex, dna: [u8; 16]) {
            Kitties::<T>::insert(kitty_id, Some(Kitty(dna)));

            Owner::<T>::insert(kitty_id, Some(owner.clone()));

            KittiesCount::<T>::put(kitty_id + 1);

            Self::deposit_event(Event::KittyCreate(owner, kitty_id));
        }
    }
}
