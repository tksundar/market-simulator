///Formats [`OrderSingle`] and [`Fill`] instances to and from any desired format
pub trait Formatter<T> {
    fn format_to(t: T) -> String;

    fn format_from(data: String) -> T;
}