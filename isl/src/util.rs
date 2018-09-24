pub trait UtilTools: Iterator {
    fn group_by_2(self, aggressive: bool) -> GroupBy2<Self>
    where
        Self: Sized,
    {
        GroupBy2 {
            iter: self,
            aggressive,
        }
    }
}

pub fn group_by_2<I>(iter: I, aggressive: bool) -> GroupBy2<I>
where
    I: Iterator,
{
    GroupBy2 { iter, aggressive }
}

pub struct GroupBy2<I>
where
    I: Iterator,
{
    iter: I,
    aggressive: bool,
}

impl<I> Iterator for GroupBy2<I>
where
    I: Iterator,
{
    type Item = (I::Item, I::Item);

    fn next(&mut self) -> Option<Self::Item> {
        let a1 = self.iter.next()?;
        let a2 = {
            let opt = self.iter.next();

            if self.aggressive {
                opt.unwrap()
            } else {
                opt?
            }
        };

        Some((a1, a2))
    }
}

impl<T: ?Sized> UtilTools for T where T: Iterator {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_by_2() {
        let vec: Vec<usize> = vec![1, 2, 3, 4, 5, 6];
        let v: Vec<(usize, usize)> = group_by_2(vec.into_iter(), false).collect();
        assert_eq!(v, vec![(1, 2), (3, 4), (5, 6)]);

        assert_eq!(
            group_by_2((Vec::new() as Vec<usize>).into_iter(), true)
                .collect::<Vec<(usize, usize)>>(),
            vec![]
        );
    }

    #[test]
    #[should_panic]
    fn test_group_by_2_panics1() {
        group_by_2(vec![1].into_iter(), true).collect::<Vec<(usize, usize)>>();
    }

    #[test]
    #[should_panic]
    fn test_group_by_2_panics2() {
        group_by_2(vec![1, 2, 3].into_iter(), true).collect::<Vec<(usize, usize)>>();
    }

    #[test]
    fn test_group_by_2_no_panic() {
        let v: Vec<(usize, usize)> = group_by_2(vec![1, 2, 3, 4, 5].into_iter(), false).collect();
        assert_eq!(v, vec![(1, 2), (3, 4)]);
    }

}
