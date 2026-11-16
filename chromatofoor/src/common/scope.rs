use crate::common::kind::Kind;
use std::{fmt, marker::PhantomData};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scope<P: ?Sized = ()>(pub PhantomData<P>);

impl fmt::Display for Scope<()> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "")
    }
}

impl<P: Kind + ?Sized> fmt::Display for Scope<P>
where
    Scope<P::Scope>: fmt::Display,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", Scope::<P::Scope>(PhantomData), P::NAME)
    }
}
