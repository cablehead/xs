use std::collections::hash_map::DefaultHasher;

use super::*;

#[test]
fn iterator() {
    let mut iter = [0, 1, 2].into_iter().double_ended_peekable();

    assert_eq!(iter.next(), Some(0));
    assert!(iter.front.is_unpeeked());
    assert!(iter.back.is_unpeeked());

    assert_eq!(iter.next(), Some(1));
    assert!(iter.front.is_unpeeked());
    assert!(iter.back.is_unpeeked());

    assert_eq!(iter.next(), Some(2));
    assert!(iter.front.is_unpeeked());
    assert!(iter.back.is_unpeeked());

    assert_eq!(iter.next(), None);
    assert!(iter.front.is_unpeeked());
    assert!(iter.back.is_unpeeked());
}

#[test]
fn double_ended_iterator() {
    let mut iter = [0, 1, 2].into_iter().double_ended_peekable();

    assert_eq!(iter.next_back(), Some(2));
    assert!(iter.front.is_unpeeked());
    assert!(iter.back.is_unpeeked());

    assert_eq!(iter.next_back(), Some(1));
    assert!(iter.front.is_unpeeked());
    assert!(iter.back.is_unpeeked());

    assert_eq!(iter.next_back(), Some(0));
    assert!(iter.front.is_unpeeked());
    assert!(iter.back.is_unpeeked());

    assert_eq!(iter.next_back(), None);
    assert!(iter.front.is_unpeeked());
    assert!(iter.back.is_unpeeked());
}

#[test]
fn peek() {
    let mut iter = [0, 1].into_iter().double_ended_peekable();

    assert_eq!(iter.peek(), Some(&0));
    assert_eq!(iter.front, MaybePeeked::Peeked(Some(0)));
    assert!(iter.back.is_unpeeked());

    assert_eq!(iter.next(), Some(0));
    assert!(iter.front.is_unpeeked());
    assert!(iter.back.is_unpeeked());

    assert_eq!(iter.peek(), Some(&1));
    assert_eq!(iter.front, MaybePeeked::Peeked(Some(1)));
    assert!(iter.back.is_unpeeked());

    assert_eq!(iter.next(), Some(1));
    assert_eq!(iter.front, MaybePeeked::Unpeeked);
    assert!(iter.back.is_unpeeked());

    assert_eq!(iter.peek(), None);
    assert_eq!(iter.front, MaybePeeked::Peeked(None));
    assert!(iter.back.is_unpeeked());

    assert_eq!(iter.next(), None);
    assert!(iter.front.is_unpeeked());
    assert!(iter.back.is_unpeeked());
}

#[test]
fn peek_mut() {
    let mut iter = [0, 1].into_iter().double_ended_peekable();

    assert_eq!(iter.peek_mut(), Some(&mut 0));
    assert_eq!(iter.front, MaybePeeked::Peeked(Some(0)));
    assert!(iter.back.is_unpeeked());

    assert_eq!(iter.next(), Some(0));
    assert!(iter.front.is_unpeeked());
    assert!(iter.back.is_unpeeked());

    assert_eq!(iter.peek_mut(), Some(&mut 1));
    assert_eq!(iter.front, MaybePeeked::Peeked(Some(1)));
    assert!(iter.back.is_unpeeked());

    assert_eq!(iter.next(), Some(1));
    assert_eq!(iter.front, MaybePeeked::Unpeeked);
    assert!(iter.back.is_unpeeked());

    assert_eq!(iter.peek_mut(), None);
    assert_eq!(iter.front, MaybePeeked::Peeked(None));
    assert!(iter.back.is_unpeeked());

    assert_eq!(iter.next(), None);
    assert!(iter.front.is_unpeeked());
    assert!(iter.back.is_unpeeked());
}

#[test]
fn peek_back() {
    let mut iter = [0, 1].into_iter().double_ended_peekable();

    assert_eq!(iter.peek_back(), Some(&1));
    assert!(iter.front.is_unpeeked());
    assert_eq!(iter.back, MaybePeeked::Peeked(Some(1)));

    assert_eq!(iter.next_back(), Some(1));
    assert!(iter.front.is_unpeeked());
    assert!(iter.back.is_unpeeked());

    assert_eq!(iter.peek_back(), Some(&0));
    assert!(iter.front.is_unpeeked());
    assert_eq!(iter.back, MaybePeeked::Peeked(Some(0)));

    assert_eq!(iter.next_back(), Some(0));
    assert!(iter.front.is_unpeeked());
    assert_eq!(iter.back, MaybePeeked::Unpeeked);

    assert_eq!(iter.peek_back(), None);
    assert!(iter.front.is_unpeeked());
    assert_eq!(iter.back, MaybePeeked::Peeked(None));

    assert_eq!(iter.next_back(), None);
    assert!(iter.front.is_unpeeked());
    assert!(iter.back.is_unpeeked());
}

#[test]
fn peek_back_mut() {
    let mut iter = [0, 1].into_iter().double_ended_peekable();

    assert_eq!(iter.peek_back_mut(), Some(&mut 1));
    assert!(iter.front.is_unpeeked());
    assert_eq!(iter.back, MaybePeeked::Peeked(Some(1)));

    assert_eq!(iter.next_back(), Some(1));
    assert!(iter.front.is_unpeeked());
    assert!(iter.back.is_unpeeked());

    assert_eq!(iter.peek_back_mut(), Some(&mut 0));
    assert!(iter.front.is_unpeeked());
    assert_eq!(iter.back, MaybePeeked::Peeked(Some(0)));

    assert_eq!(iter.next_back(), Some(0));
    assert!(iter.front.is_unpeeked());
    assert_eq!(iter.back, MaybePeeked::Unpeeked);

    assert_eq!(iter.peek_back_mut(), None);
    assert!(iter.front.is_unpeeked());
    assert_eq!(iter.back, MaybePeeked::Peeked(None));

    assert_eq!(iter.next_back(), None);
    assert!(iter.front.is_unpeeked());
    assert!(iter.back.is_unpeeked());
}

#[test]
fn peek_and_peek_back_forward() {
    let mut iter = [0, 1, 2].into_iter().double_ended_peekable();
    assert_eq!(iter.peek(), Some(&0));
    assert_eq!(iter.front, MaybePeeked::Peeked(Some(0)));
    assert_eq!(iter.back, MaybePeeked::Unpeeked);

    assert_eq!(iter.peek_back(), Some(&2));
    assert_eq!(iter.front, MaybePeeked::Peeked(Some(0)));
    assert_eq!(iter.back, MaybePeeked::Peeked(Some(2)));

    assert_eq!(iter.next(), Some(0));
    assert_eq!(iter.front, MaybePeeked::Unpeeked);
    assert_eq!(iter.back, MaybePeeked::Peeked(Some(2)));

    assert_eq!(iter.peek(), Some(&1));
    assert_eq!(iter.front, MaybePeeked::Peeked(Some(1)));
    assert_eq!(iter.back, MaybePeeked::Peeked(Some(2)));

    assert_eq!(iter.next(), Some(1));
    assert_eq!(iter.front, MaybePeeked::Unpeeked);
    assert_eq!(iter.back, MaybePeeked::Peeked(Some(2)));

    assert_eq!(iter.peek(), Some(&2));
    assert_eq!(iter.front, MaybePeeked::Peeked(None));
    assert_eq!(iter.back, MaybePeeked::Peeked(Some(2)));

    assert_eq!(iter.next(), Some(2));
    assert_eq!(iter.front, MaybePeeked::Unpeeked);
    assert_eq!(iter.back, MaybePeeked::Unpeeked);

    assert_eq!(iter.peek(), None);
    assert_eq!(iter.front, MaybePeeked::Peeked(None));
    assert_eq!(iter.back, MaybePeeked::Unpeeked);

    assert_eq!(iter.peek_back(), None);
    assert_eq!(iter.front, MaybePeeked::Peeked(None));
    assert_eq!(iter.back, MaybePeeked::Peeked(None));
}

#[test]
fn peek_and_peek_back_backward() {
    let mut iter = [0, 1, 2].into_iter().double_ended_peekable();
    assert_eq!(iter.peek(), Some(&0));
    assert_eq!(iter.front, MaybePeeked::Peeked(Some(0)));
    assert_eq!(iter.back, MaybePeeked::Unpeeked);

    assert_eq!(iter.peek_back(), Some(&2));
    assert_eq!(iter.front, MaybePeeked::Peeked(Some(0)));
    assert_eq!(iter.back, MaybePeeked::Peeked(Some(2)));

    assert_eq!(iter.next_back(), Some(2));
    assert_eq!(iter.front, MaybePeeked::Peeked(Some(0)));
    assert_eq!(iter.back, MaybePeeked::Unpeeked);

    assert_eq!(iter.peek_back(), Some(&1));
    assert_eq!(iter.front, MaybePeeked::Peeked(Some(0)));
    assert_eq!(iter.back, MaybePeeked::Peeked(Some(1)));

    assert_eq!(iter.next_back(), Some(1));
    assert_eq!(iter.front, MaybePeeked::Peeked(Some(0)));
    assert_eq!(iter.back, MaybePeeked::Unpeeked);

    assert_eq!(iter.peek_back(), Some(&0));
    assert_eq!(iter.front, MaybePeeked::Peeked(Some(0)));
    assert_eq!(iter.back, MaybePeeked::Peeked(None));

    assert_eq!(iter.next_back(), Some(0));
    assert_eq!(iter.front, MaybePeeked::Unpeeked);
    assert_eq!(iter.back, MaybePeeked::Unpeeked);

    assert_eq!(iter.peek(), None);
    assert_eq!(iter.front, MaybePeeked::Peeked(None));
    assert_eq!(iter.back, MaybePeeked::Unpeeked);

    assert_eq!(iter.peek_back(), None);
    assert_eq!(iter.front, MaybePeeked::Peeked(None));
    assert_eq!(iter.back, MaybePeeked::Peeked(None));
}

#[test]
fn next_if() {
    let mut iter = [0, 1, 2, 3].into_iter().double_ended_peekable();

    assert_eq!(iter.next_if(|x| x == &0), Some(0));
    assert!(iter.front.is_unpeeked());
    assert!(iter.back.is_unpeeked());

    assert_eq!(iter.peek(), Some(&1));
    assert!(iter.next_if(|x| x == &42).is_none());
    assert_eq!(iter.front, MaybePeeked::Peeked(Some(1)));
    assert!(iter.back.is_unpeeked());

    assert_eq!(iter.next_if(|x| x == &1), Some(1));
    assert!(iter.front.is_unpeeked());
    assert!(iter.back.is_unpeeked());

    assert_eq!(iter.peek_back(), Some(&3));
    assert!(iter.next_if(|x| x == &42).is_none());
    assert_eq!(iter.front, MaybePeeked::Peeked(Some(2)));
    assert_eq!(iter.back, MaybePeeked::Peeked(Some(3)));

    assert_eq!(iter.next_if(|x| x == &2), Some(2));
    assert_eq!(iter.front, MaybePeeked::Unpeeked);
    assert_eq!(iter.back, MaybePeeked::Peeked(Some(3)));

    assert!(iter.next_if(|x| x == &42).is_none());
    assert_eq!(iter.front, MaybePeeked::Peeked(Some(3)));
    assert!(iter.back.is_unpeeked());

    assert_eq!(iter.next_if(|x| x == &3), Some(3));
    assert!(iter.front.is_unpeeked());
    assert!(iter.back.is_unpeeked());

    assert!(iter.peek().is_none());
    assert!(iter.next_if(|x| x == &42).is_none());
    assert_eq!(iter.front, MaybePeeked::Peeked(None));
    assert!(iter.back.is_unpeeked());

    assert!(iter.next().is_none());
    assert!(iter.front.is_unpeeked());
    assert!(iter.back.is_unpeeked());

    assert!(iter.peek_back().is_none());
    assert!(iter.next_if(|_| unreachable!()).is_none());
    assert_eq!(iter.front, MaybePeeked::Peeked(None));
    assert!(iter.back.is_unpeeked());
}

#[test]
fn next_back_if() {
    let mut iter = [0, 1, 2, 3].into_iter().double_ended_peekable();

    assert_eq!(iter.next_back_if(|x| x == &3), Some(3));
    assert!(iter.front.is_unpeeked());
    assert!(iter.back.is_unpeeked());

    assert_eq!(iter.peek_back(), Some(&2));
    assert!(iter.next_back_if(|x| x == &42).is_none());
    assert!(iter.front.is_unpeeked());
    assert_eq!(iter.back, MaybePeeked::Peeked(Some(2)));

    assert_eq!(iter.next_back_if(|x| x == &2), Some(2));
    assert!(iter.front.is_unpeeked());
    assert!(iter.back.is_unpeeked());

    assert_eq!(iter.peek(), Some(&0));
    assert!(iter.next_back_if(|x| x == &42).is_none());
    assert_eq!(iter.front, MaybePeeked::Peeked(Some(0)));
    assert_eq!(iter.back, MaybePeeked::Peeked(Some(1)));

    assert_eq!(iter.next_back_if(|x| x == &1), Some(1));
    assert_eq!(iter.front, MaybePeeked::Peeked(Some(0)));
    assert_eq!(iter.back, MaybePeeked::Unpeeked);

    assert!(iter.next_back_if(|x| x == &42).is_none());
    assert!(iter.front.is_unpeeked());
    assert_eq!(iter.back, MaybePeeked::Peeked(Some(0)));

    assert_eq!(iter.next_back_if(|x| x == &0), Some(0));
    assert!(iter.front.is_unpeeked());
    assert!(iter.back.is_unpeeked());

    assert!(iter.peek_back().is_none());
    assert!(iter.next_back_if(|_| unreachable!()).is_none());
    assert!(iter.front.is_unpeeked());
    assert_eq!(iter.back, MaybePeeked::Peeked(None));

    assert!(iter.next_back().is_none());
    assert!(iter.front.is_unpeeked());
    assert!(iter.back.is_unpeeked());

    assert!(iter.peek().is_none());
    assert!(iter.next_back_if(|_| unreachable!()).is_none());
    assert!(iter.front.is_unpeeked());
    assert_eq!(iter.back, MaybePeeked::Peeked(None));
}

#[test]
fn next_if_eq() {
    let mut iter = [0, 1, 2].into_iter().double_ended_peekable();

    assert_eq!(iter.next_if_eq(&0), Some(0));
    assert!(iter.front.is_unpeeked());
    assert!(iter.back.is_unpeeked());

    assert_eq!(iter.next_if_eq(&42), None);
    assert_eq!(iter.front, MaybePeeked::Peeked(Some(1)));
    assert!(iter.back.is_unpeeked());

    assert_eq!(iter.peek_back(), Some(&2));
    assert_eq!(iter.next_if_eq(&1), Some(1));
    assert_eq!(iter.front, MaybePeeked::Unpeeked);
    assert_eq!(iter.back, MaybePeeked::Peeked(Some(2)));

    assert_eq!(iter.next_if_eq(&42), None);
    assert_eq!(iter.front, MaybePeeked::Peeked(Some(2)));
    assert_eq!(iter.back, MaybePeeked::Unpeeked);

    assert_eq!(iter.next_if_eq(&2), Some(2));
    assert_eq!(iter.front, MaybePeeked::Unpeeked);
    assert_eq!(iter.back, MaybePeeked::Unpeeked);

    assert_eq!(iter.next_if_eq(&42), None);
    assert_eq!(iter.front, MaybePeeked::Peeked(None));
    assert_eq!(iter.back, MaybePeeked::Unpeeked);
}

#[test]
fn next_back_if_eq() {
    let mut iter = [0, 1, 2].into_iter().double_ended_peekable();

    assert_eq!(iter.next_back_if_eq(&2), Some(2));
    assert!(iter.front.is_unpeeked());
    assert!(iter.back.is_unpeeked());

    assert_eq!(iter.next_back_if_eq(&42), None);
    assert!(iter.front.is_unpeeked());
    assert_eq!(iter.back, MaybePeeked::Peeked(Some(1)));

    assert_eq!(iter.peek(), Some(&0));
    assert_eq!(iter.next_back_if_eq(&1), Some(1));
    assert_eq!(iter.front, MaybePeeked::Peeked(Some(0)));
    assert_eq!(iter.back, MaybePeeked::Unpeeked);

    assert_eq!(iter.next_back_if_eq(&42), None);
    assert_eq!(iter.front, MaybePeeked::Unpeeked);
    assert_eq!(iter.back, MaybePeeked::Peeked(Some(0)));

    assert_eq!(iter.next_back_if_eq(&0), Some(0));
    assert_eq!(iter.front, MaybePeeked::Unpeeked);
    assert_eq!(iter.back, MaybePeeked::Unpeeked);

    assert_eq!(iter.next_back_if_eq(&42), None);
    assert_eq!(iter.front, MaybePeeked::Unpeeked);
    assert_eq!(iter.back, MaybePeeked::Peeked(None));
}

#[test]
fn next_front_back_if_even() {
    let mut iter = [0, 1, 2, 3, 4, 5].into_iter().double_ended_peekable();
    assert_eq!(
        iter.next_front_back_if(|a, b| a == &0 && b == &5),
        Some((0, 5))
    );
    assert!(iter.front.is_unpeeked());
    assert!(iter.back.is_unpeeked());

    assert!(iter
        .next_front_back_if(|a, b| a == &1 && b == &42)
        .is_none());
    assert_eq!(iter.front, MaybePeeked::Peeked(Some(1)));
    assert_eq!(iter.back, MaybePeeked::Peeked(Some(4)));

    assert_eq!(
        iter.next_front_back_if(|a, b| a == &1 && b == &4),
        Some((1, 4))
    );
    assert!(iter.front.is_unpeeked());
    assert!(iter.back.is_unpeeked());

    assert_eq!(iter.peek_back(), Some(&3));
    assert_eq!(
        iter.next_front_back_if(|a, b| a == &2 && b == &3),
        Some((2, 3))
    );
    assert!(iter.front.is_unpeeked());
    assert!(iter.back.is_unpeeked());

    assert!(iter.next_front_back_if(|_, _| unreachable!()).is_none());
    assert_eq!(iter.front, MaybePeeked::Peeked(None));
    assert_eq!(iter.back, MaybePeeked::Peeked(None));
}

#[test]
fn next_front_back_if_odd() {
    let mut iter = [0, 1, 2].into_iter().double_ended_peekable();
    assert_eq!(
        iter.next_front_back_if(|a, b| a == &0 && b == &2),
        Some((0, 2))
    );
    assert!(iter.front.is_unpeeked());
    assert!(iter.back.is_unpeeked());

    assert!(iter.next_front_back_if(|_, _| unreachable!()).is_none());
    assert_eq!(iter.front, MaybePeeked::Peeked(Some(1)));
    assert_eq!(iter.back, MaybePeeked::Peeked(None));

    assert!(iter.next_front_back_if(|_, _| unreachable!()).is_none());
    assert_eq!(iter.front, MaybePeeked::Peeked(Some(1)));
    assert_eq!(iter.back, MaybePeeked::Peeked(None));
}

#[test]
fn next_front_back_if_eq() {
    let mut iter = [0, 1, 2, 3].into_iter().double_ended_peekable();

    assert_eq!(iter.next_front_back_if_eq(&0, &3), Some((0, 3)));
    assert!(iter.front.is_unpeeked());
    assert!(iter.back.is_unpeeked());

    assert_eq!(iter.peek_back(), Some(&2));
    assert_eq!(iter.next_front_back_if_eq(&1, &42), None);
    assert_eq!(iter.front, MaybePeeked::Peeked(Some(1)));
    assert_eq!(iter.back, MaybePeeked::Peeked(Some(2)));

    assert_eq!(iter.next_front_back_if_eq(&1, &2), Some((1, 2)));
    assert_eq!(iter.front, MaybePeeked::Unpeeked);
    assert_eq!(iter.back, MaybePeeked::Unpeeked);

    assert_eq!(iter.next_front_back_if_eq(&42, &42), None);
    assert_eq!(iter.front, MaybePeeked::Peeked(None));
    assert_eq!(iter.back, MaybePeeked::Peeked(None));
}

#[test]
fn size_hint() {
    let mut iter = [0, 1, 2, 3].into_iter().double_ended_peekable();
    assert_eq!(iter.size_hint(), (4, Some(4)));

    assert_eq!(iter.peek(), Some(&0));
    assert_eq!(iter.size_hint(), (4, Some(4)));

    assert_eq!(iter.peek_back(), Some(&3));
    assert_eq!(iter.size_hint(), (4, Some(4)));

    assert_eq!(iter.next(), Some(0));
    assert_eq!(iter.size_hint(), (3, Some(3)));

    let mut iter = [0, 1, 2, 3]
        .into_iter()
        .filter(|x| x % 2 == 0)
        .double_ended_peekable();
    assert_eq!(iter.size_hint(), (0, Some(4)));

    assert_eq!(iter.peek(), Some(&0));
    assert_eq!(iter.size_hint(), (1, Some(4)));

    assert_eq!(iter.peek_back(), Some(&2));
    assert_eq!(iter.size_hint(), (2, Some(3)));

    assert_eq!(iter.next(), Some(0));
    assert_eq!(iter.size_hint(), (1, Some(2)));

    assert_eq!(iter.next_back(), Some(2));
    assert_eq!(iter.size_hint(), (0, Some(1)));
}

#[test]
fn clone() {
    let mut iter = [0, 1, 2, 3].into_iter().double_ended_peekable();
    assert_eq!(iter.next(), Some(0));
    assert_eq!(iter.peek(), Some(&1));
    assert_eq!(iter.peek_back(), Some(&3));

    {
        let mut iter = iter.clone();
        assert_eq!(iter.next(), Some(1));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), Some(3));
        assert_eq!(iter.next(), None);
    }

    assert_eq!(iter.next(), Some(1));
    assert_eq!(iter.next(), Some(2));
    assert_eq!(iter.next(), Some(3));
    assert_eq!(iter.next(), None);
}

#[test]
fn debug() {
    let mut iter = [0, 1, 2, 3].into_iter().double_ended_peekable();
    assert_eq!(iter.next(), Some(0));
    assert_eq!(iter.peek(), Some(&1));
    assert_eq!(iter.peek_back(), Some(&3));

    assert_eq!(
        format!("{iter:?}"),
        "DoubleEndedPeekable { iter: IntoIter([2]), front: Peeked(Some(1)), back: Peeked(Some(3)) }",
    );
}

#[test]
fn partial_eq() {
    let mut iter = (0..5).double_ended_peekable();
    assert_eq!(iter.next(), Some(0));
    assert_eq!(iter.peek(), Some(&1));
    assert_eq!(iter.peek_back(), Some(&4));

    assert_eq!(
        iter,
        DoubleEndedPeekable {
            iter: 2..4,
            front: MaybePeeked::Peeked(Some(1)),
            back: MaybePeeked::Peeked(Some(4))
        },
    );
}

#[test]
fn hash() {
    let mut iter = (0..5).double_ended_peekable();
    assert_eq!(iter.next(), Some(0));
    assert_eq!(iter.peek(), Some(&1));
    assert_eq!(iter.peek_back(), Some(&4));

    let mut hasher = DefaultHasher::default();
    iter.hash(&mut hasher);
    let hash = hasher.finish();

    let mut hasher = DefaultHasher::default();
    (2..4).hash(&mut hasher);
    MaybePeeked::Peeked(Some(1)).hash(&mut hasher);
    MaybePeeked::Peeked(Some(4)).hash(&mut hasher);
    let expected_hash = hasher.finish();

    assert_eq!(hash, expected_hash);
}
