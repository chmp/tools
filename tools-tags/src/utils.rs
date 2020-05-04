/// A helper trait to ignore results
pub trait Ignorable {
    fn ignore(self);
}

impl<T, E> Ignorable for std::result::Result<T, E> {
    fn ignore(self) {
        // DO NOTHING
    }
}

pub trait ResultsCollector<T, E> {
    fn collect_results(self) -> (Vec<T>, Vec<E>);

    fn collect_results_transformed<A, F, B, G>(self, f: F, g: G) -> (Vec<A>, Vec<B>)
    where
        F: FnMut(T) -> A,
        G: FnMut(E) -> B;
}

impl<U, T, E> ResultsCollector<T, E> for U
where
    U: Iterator<Item = Result<T, E>>,
{
    fn collect_results(self) -> (Vec<T>, Vec<E>) {
        self.collect_results_transformed(|v| v, |v| v)
    }

    fn collect_results_transformed<A, F, B, G>(self, f: F, g: G) -> (Vec<A>, Vec<B>)
    where
        F: FnMut(T) -> A,
        G: FnMut(E) -> B,
    {
        let mut f = f;
        let mut g = g;
        let mut oks = Vec::<A>::new();
        let mut errs = Vec::<B>::new();

        for result in self {
            match result {
                Ok(v) => oks.push(f(v)),
                Err(v) => errs.push(g(v)),
            }
        }

        (oks, errs)
    }
}
