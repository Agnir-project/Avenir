pub trait With<T> {
    fn with(self, data: T) -> Self;
}

pub trait Build<T> {
    fn build(self) -> T;
}
