pub mod alldebrid;
pub mod debrid_link;
pub mod premiumize;
pub mod real_debrid;
pub mod torbox;

pub use alldebrid::{ALLDEBRID_SECRET_KEY, AllDebridProvider};
pub use debrid_link::{DEBRID_LINK_SECRET_KEY, DebridLinkProvider};
pub use premiumize::{PREMIUMIZE_SECRET_KEY, PremiumizeProvider};
pub use real_debrid::{
    REAL_DEBRID_CLIENT_ID_KEY, REAL_DEBRID_CLIENT_SECRET_KEY, REAL_DEBRID_REFRESH_KEY,
    REAL_DEBRID_SECRET_KEY, RealDebridProvider,
};
pub use torbox::{TORBOX_SECRET_KEY, TorBoxProvider};
