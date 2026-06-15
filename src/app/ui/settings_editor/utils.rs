pub trait EditorUi<T> {
    fn new(data: T) -> Self;

    fn is_valid(&self) -> bool {
        self.parse().is_some()
    }

    fn parse(&self) -> Option<T>;
}
