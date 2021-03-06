//! Service that interfaces with the browser to handle routing.

use stdweb::{
    web::{event::PopStateEvent, window, EventListenerHandle, History, IEventTarget, Location},
    Value,
};
use yew::callback::Callback;

use crate::route::{Route, RouteState};
use std::marker::PhantomData;
use stdweb::{
    js,
    unstable::{TryFrom, TryInto},
};

/// A service that facilitates manipulation of the browser's URL bar and responding to browser events
/// when users press 'forward' or 'back'.
///
/// The `T` determines what route state can be stored in the route service.
#[derive(Debug)]
pub struct RouteService<STATE = ()> {
    history: History,
    location: Location,
    event_listener: Option<EventListenerHandle>,
    phantom_data: PhantomData<STATE>,
}

impl<STATE> Default for RouteService<STATE>
where
    STATE: RouteState,
{
    fn default() -> Self {
        RouteService::<STATE>::new()
    }
}

impl<T> RouteService<T> {
    /// Creates the route service.
    pub fn new() -> RouteService<T> {
        let location = window()
            .location()
            .expect("browser does not support location API");
        RouteService {
            history: window().history(),
            location,
            event_listener: None,
            phantom_data: PhantomData,
        }
    }

    #[inline]
    fn get_route_from_location(location: &Location) -> String {
        let path = location.pathname().unwrap();
        let query = location.search().unwrap();
        let fragment = location.hash().unwrap();
        format_route_string(&path, &query, &fragment)
    }

    /// Gets the path name of the current url.
    pub fn get_path(&self) -> String {
        self.location.pathname().unwrap()
    }

    /// Gets the query string of the current url.
    pub fn get_query(&self) -> String {
        self.location.search().unwrap()
    }

    /// Gets the fragment of the current url.
    pub fn get_fragment(&self) -> String {
        self.location.hash().unwrap()
    }
}

impl<STATE> RouteService<STATE>
where
    STATE: RouteState,
{
    /// Registers a callback to the route service.
    /// Callbacks will be called when the History API experiences a change such as
    /// popping a state off of its stack when the forward or back buttons are pressed.
    pub fn register_callback(&mut self, callback: Callback<Route<STATE>>) {
        self.event_listener = Some(window().add_event_listener(move |event: PopStateEvent| {
            let state_value: Value = event.state();
            let state_string: String = String::try_from(state_value).unwrap_or_default();
            let state: STATE = serde_json::from_str(&state_string).unwrap_or_else(|_| {
                log::error!("Could not deserialize state string");
                STATE::default()
            });


            // Can't use the existing location, because this is a callback, and can't move it in
            // here.
            let location: Location = window().location().unwrap();
            let route: String = Self::get_route_from_location(&location);


            callback.emit(Route { route, state })
        }));
    }

    /// Sets the browser's url bar to contain the provided route,
    /// and creates a history entry that can be navigated via the forward and back buttons.
    ///
    /// The route should be a relative path that starts with a `/`.
    pub fn set_route(&mut self, route: &str, state: STATE) {
        let state_string: String = serde_json::to_string(&state).unwrap_or_else(|_| {
            log::error!("Could not serialize state string");
            "".to_string()
        });
        self.history.push_state(state_string, "", Some(route));
    }

    /// Replaces the route with another one removing the most recent history event and
    /// creating another history event in its place.
    pub fn replace_route(&mut self, route: &str, state: STATE) {
        let state_string: String = serde_json::to_string(&state).unwrap_or_else(|_| {
            log::error!("Could not serialize state string");
            "".to_string()
        });
        let _ = self.history.replace_state(state_string, "", Some(route));
    }

    /// Gets the concatenated path, query, and fragment.
    pub fn get_route(&self) -> Route<STATE> {
        let route_string = Self::get_route_from_location(&self.location);
        let state: STATE = get_state_string(&self.history)
            .or_else(|| {
                log::trace!("History state is empty");
                None
            })
            .and_then(|state_string| -> Option<Option<STATE>>{
                serde_json::from_str(&state_string)
                    .ok()
                    .or_else(|| {
                        log::error!("Could not deserialize state string");
                        None
                    })
            })
            .and_then(std::convert::identity) // flatten
            .unwrap_or_default();
        Route {
            route: route_string,
            state,
        }
    }
}


/// Formats a path, query, and fragment into a string.
///
/// # Note
/// This expects that all three already have their expected separators (?, #, etc)
pub(crate) fn format_route_string(path: &str, query: &str, fragment: &str) -> String {
    format!(
        "{path}{query}{fragment}",
        path = path,
        query = query,
        fragment = fragment
    )
}


fn get_state(history: &History) -> Value {
    js!(
        return @{history}.state;
    )
}

fn get_state_string(history: &History) -> Option<String> {
    get_state(history).try_into().ok()
}
