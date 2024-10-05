use std::fmt;

pub struct DisplaySeparated<'a, T, I, F>
where
    F: Fn(&'_ mut fmt::Formatter, &I) -> fmt::Result,
    &'a T: IntoIterator<Item = I>,
{
    list: &'a T,
    separator: &'static str,
    print: F,
}

pub fn display_comma_separated<'a, T, I>(
    list: &'a T,
) -> DisplaySeparated<'a, T, I, impl Fn(&mut fmt::Formatter, &I) -> fmt::Result>
where
    &'a T: IntoIterator<Item = I>,
    I: fmt::Display,
{
    DisplaySeparated {
        list,
        separator: ", ",
        print: |f, i| write!(f, "{i}"),
    }
}
pub fn display_period_separated<'a, T, I>(
    list: &'a T,
) -> DisplaySeparated<'a, T, I, impl Fn(&mut fmt::Formatter, &I) -> fmt::Result>
where
    &'a T: IntoIterator<Item = I>,
    I: fmt::Display,
{
    DisplaySeparated {
        list,
        separator: ".",
        print: |f, i| write!(f, "{i}"),
    }
}

pub fn display_separated<'a, T, I, F>(
    list: &'a T,
    separator: &'static str,
    print: F,
) -> DisplaySeparated<'a, T, I, F>
where
    &'a T: IntoIterator<Item = I>,
    F: Fn(&'_ mut fmt::Formatter, &I) -> fmt::Result,
{
    DisplaySeparated {
        list,
        separator,
        print,
    }
}

impl<'a, T, I, F> fmt::Display for DisplaySeparated<'a, T, I, F>
where
    &'a T: IntoIterator<Item = I>,
    F: Fn(&mut fmt::Formatter, &I) -> fmt::Result,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        for item in self.list {
            if first {
                first = false
            } else {
                write!(f, "{}", self.separator)?;
            }
            (self.print)(f, &item)?;
        }
        Ok(())
    }
}
