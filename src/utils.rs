pub trait With<T> {
    fn with(self, data: T) -> Self;
}

pub trait WithError<T, E>
where
    Self: std::marker::Sized
{

    fn with_error(self, data: T) -> Result<Self, E>;
}

pub trait Build<T> {
    fn build(self) -> T;
}
