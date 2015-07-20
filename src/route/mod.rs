use std::path::PathBuf;

use input::Input;
use output::Output;
use service::ServiceAccess;
use service::StaticServices;

use hyper::server::request::Request as HyperRequest;
use hyper::server::response::Response as HyperResponse;
use hyper::method::Method as HyperMethod;

pub struct Route {
    pub url: String,
    pub method: MethodsMask,
    pub handler: Handler,
}

impl Route {
    /// Returns true if this route can handle the given request.
    pub fn matches(&self, request: &HyperRequest) -> bool {
        if !self.method.matches(&request.method) {
            return false;
        }

        true        // FIXME: 
    }
}

/// Describes which methods must be used by the request for a route to be used.
pub struct MethodsMask {
    /// True if the `GET` method matches this mask.
    pub get: bool,
    /// True if the `POST` method matches this mask.
    pub post: bool,
    /// True if the `PUT` method matches this mask.
    pub put: bool,
    /// True if the `DELETE` method matches this mask.
    pub delete: bool,
}

impl MethodsMask {
    /// Parses from a string of the `route!` macro.
    pub fn parse(_: &str) -> MethodsMask {
        // FIXME:
        MethodsMask {
            get: true,
            post: false,
            put: false,
            delete: false,
        }
    }

    /// Returns true if the mask contains the specified method.
    pub fn matches(&self, method: &HyperMethod) -> bool {
        match method {
            &HyperMethod::Get => self.get,
            &HyperMethod::Post => self.post,
            &HyperMethod::Put => self.put,
            &HyperMethod::Delete => self.delete,
            _ => false
        }
    }
}

/// Describes how to handle a route.
pub enum Handler {
    Static(PathBuf),
    Dynamic(Box<DynamicHandler + Send + Sync>),
}

pub struct Router {
    /// List of the routes to try to match. They will be tried in this order.
    pub routes: Vec<Route>,
}

/// Describes types that can process a route.
pub trait DynamicHandler {
    /// Handles a request.
    fn call(&self, HyperRequest, HyperResponse, &StaticServices);
}

impl<I, O> DynamicHandler for fn(I) -> O where I: Input, O: Output {
    fn call(&self, request: HyperRequest, response: HyperResponse, services: &StaticServices) {
        let input = match I::process(request) {
            Ok(i) => i,
            Err(_) => return        // TODO: handle properly
        };

        let output = (*self)(input);
        output.send(response, services);
    }
}

impl<I, O, S1> DynamicHandler for fn(I, S1) -> O
                                  where I: Input, O: Output, S1: for<'s> ServiceAccess<'s>
{
    fn call(&self, request: HyperRequest, response: HyperResponse, services: &StaticServices) {
        let input = match I::process(request) {
            Ok(i) => i,
            Err(_) => return        // TODO: handle properly
        };

        let s1 = S1::load(services);

        let output = (*self)(input, s1);
        output.send(response, services);
    }
}

#[macro_export]
macro_rules! router {
    (__parse_route $routes:ident [$($ex:tt)*] / $($t:tt)+) => (
        {
            let method = router!(__parse_method $($ex)*);
            router!(__parse_route2 $routes method [/] $($t)+)
        }
    );

    (__parse_route $routes:ident [$($ex:tt)*] $f:tt $($t:tt)*) => (
        router!(__parse_route $routes [$($ex)* $f] $($t)*)
    );

    (__parse_route $routes:ident []) => ();

    (__parse_route2 $routes:ident $method:ident [$($ex:tt)*] => $handler:expr, $($t:tt)*) => (
        {
            let pattern = router!(__parse_pattern $($ex)*);
            let handler = Box::new($handler) as Box<$crate::route::DynamicHandler + Send + Sync>;
            $routes.push($crate::route::Route {
                url: pattern,
                method: $method,
                handler: $crate::route::Handler::Dynamic(handler),
            });

            router!(__parse_route $routes [] $($t)*);
        }
    );

    (__parse_route2 $routes:ident $method:ident [$($ex:tt)*] => $handler:expr) => (
        router!(__parse_route2 $routes $method [$($ex)*] => $handler,)
    );

    (__parse_route2 $routes:ident $method:ident [$($ex:tt)*] $f:tt $($t:tt)*) => (
        router!(__parse_route2 $routes $method [$($ex)* $f] $($t)*)
    );

    (__parse_pattern $($t:tt)*) => (
        // TODO: 
        "".to_string()
    );

    (__parse_method $($t:tt)*) => (
        // TODO: 
        $crate::route::MethodsMask { get: true, post: false, put: false, delete: false }
    );

    // main route
    ($($t:tt)+) => (
        {
            let mut routes = Vec::new();
            router!(__parse_route routes [] $($t)+);
            $crate::route::Router { routes: routes }
        }
    );
}
