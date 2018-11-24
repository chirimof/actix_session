extern crate actix_web;
extern crate futures;

use actix_web::{
    middleware::{
        Middleware,
        Started as MdStarted,
        Response as MdResponse,
    },
    HttpRequest,
    HttpResponse,
    error::Error as ActixWebError,
};

use futures::future::Future;

use std::marker::PhantomData;
use std::rc::Rc;
use std::cell::{RefCell};
use std::ops::{Deref};
use std::clone::Clone;


pub struct Session<M> {
    pub session_manager: Rc<RefCell<M>>
}

impl<M> Clone for Session<M> {
    fn clone(&self) -> Self {
        let session_manager = Rc::clone(&self.session_manager);
        Session { session_manager }
    }
}

impl<M: SessionManager> Session<M> {
    fn new(session_manager: Rc<RefCell<M>>) -> Self {
        Session { session_manager }
    }

    pub fn changed(&self) -> bool {
        self.session_manager.borrow().changed()
    }
}

// Session Middleware
pub struct SessionMiddleware<B, St, Sm> {
    backend: B,
    state: PhantomData<St>,
    session_manager: PhantomData<Sm>
}

impl<B, St, Sm> SessionMiddleware<B, St, Sm>
    where
        B: SessionBackend<St>,
        St: 'static,
        Sm: 'static,
{
    pub fn new(backend: B) -> Self {
        SessionMiddleware {
            backend,
            state: PhantomData,
            session_manager: PhantomData
        }
    }
}

impl<B, St, Sm> Middleware<St> for SessionMiddleware<B, St, Sm>
    where
        B: SessionBackend<St>,
        St: 'static,
        Sm: 'static,
{
    fn start(&self, req: &HttpRequest<St>) -> Result<MdStarted, ActixWebError> {
        let req_clone = req.clone();
        let get_futu =
            self.backend.get_session(req)
                .and_then(move |session_manager| {
                    req_clone.extensions_mut().insert(
                        // Session<type of session_manager>
                        Session::new(Rc::new(RefCell::new(session_manager)))
                    );
                    Ok(None)
                });
        Ok(MdStarted::Future(Box::new(get_futu)))
    }

    fn response(&self, req: &HttpRequest<St>, resp: HttpResponse) -> Result<MdResponse, ActixWebError> {
        Ok(self.backend.response(req, resp))
    }
}

// It orders getting(to get from backend) and updating(to update for backend)
// When middleware-started get_session will be used and return future that resolve session manager
// When middleware-response the (if necessary) update_session will be used and update the changes
// This trait delegate the implementer to decide contents of the Response
// because update_session may be failed
pub trait SessionBackend<St>: 'static {

    type Manager: SessionManager;
    type GetFuture: Future<Item = Self::Manager, Error = ActixWebError>;

    fn get_session(&self, req: &HttpRequest<St>) -> Self::GetFuture;

    fn update_session(&self, session_manager: &Self::Manager, resp: HttpResponse) -> MdResponse;

    // In default, if session manager was set and it was changed, update_session will be called
    fn response(&self, req: &HttpRequest<St>, resp: HttpResponse) -> MdResponse {
        if let Some(session) = req.session::<Self::Manager>() {
            if session.borrow().changed() {
                return self.update_session(session.borrow().deref(), resp)
            }
        }
        MdResponse::Done(resp)
    }
}

pub trait SessionManager: 'static {
    type SessionData;

    fn get_session(&self) -> &Self::SessionData;

    fn get_mut_session(&mut self) -> &mut Self::SessionData;

    fn changed(&self) -> bool;


}

pub trait UseSession
{
    fn session<M: SessionManager> (&self) -> Option<Rc<RefCell<M>>>;
}

impl<S> UseSession for HttpRequest<S>
{
    fn session<M: SessionManager> (&self) -> Option<Rc<RefCell<M>>> {
        self.extensions()
            .get::<Session<M>>()
            .map(|session: &Session<M>| session.clone().session_manager)
    }
}


#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
