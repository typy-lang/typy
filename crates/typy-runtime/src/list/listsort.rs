//! The sort of `list.sort` and `sorted`, a port of CPython 3.15's `listsort` (OI-27): natural
//! runs that `count_run` finds, extended by binary insertion to the `minrun` that
//! `minrun_next` spreads over the list, then merged by the powersort strategy, galloping when
//! one run keeps winning. It calls `less` with the same items in the same order as CPython
//! calls `__lt__`, so a method that prints or raises shows the same calls. A comparison that
//! raises ends the sort at once and leaves the items a permutation of what they were, as
//! CPython leaves them.

/// The number of wins in a row that starts galloping.
const MIN_GALLOP: usize = 7;
/// The largest `minrun`.
const MAX_MINRUN: usize = 64;
/// The most runs waiting to be merged: the powersort strategy keeps fewer than one for each
/// bit of a length.
const MAX_PENDING: usize = 64;

/// A comparison raised; the exception waits with the runtime (ADR-0039).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Raised;

/// `a < b`, or `Raised`.
pub(crate) type Less<'less, T> = &'less dyn Fn(T, T) -> Result<bool, Raised>;

/// A run waiting to be merged: its first position, its length, and its node level in the
/// powersort merge tree.
#[derive(Clone, Copy, Default)]
struct Run {
    start: usize,
    length: usize,
    power: u32,
}

/// What one sort keeps between its steps, CPython's `MergeState`.
struct MergeState<'sort, T> {
    less: Less<'sort, T>,
    min_gallop: usize,
    runs: [Run; MAX_PENDING],
    count: usize,
    length: usize,
    /// The state of `minrun_next`: the running sum, the shift and its mask.
    current: usize,
    shift: u32,
    mask: usize,
}

/// Sorts `items` stably with `less`, which a comparison that raises ends; `scratch` holds at
/// least half the items, rounded up.
pub(crate) fn sort<T: Copy>(
    items: &mut [T],
    scratch: &mut [T],
    less: Less<'_, T>,
) -> Result<(), Raised> {
    let length = items.len();
    if length < 2 {
        return Ok(());
    }
    let mut shift = 0;
    while length >> shift >= MAX_MINRUN {
        shift += 1;
    }
    let mut state = MergeState {
        less,
        min_gallop: MIN_GALLOP,
        runs: [Run::default(); MAX_PENDING],
        count: 0,
        length,
        current: 0,
        shift,
        mask: (1 << shift) - 1,
    };
    let (mut start, mut remaining) = (0, length);
    while remaining > 0 {
        let mut run = count_run(&mut items[start..], remaining, less)?;
        let minrun = state.minrun_next();
        if run < minrun {
            let forced = remaining.min(minrun);
            binary_sort(&mut items[start..start + forced], run, less)?;
            run = forced;
        }
        state.found_new_run(items, scratch, run)?;
        state.runs[state.count] = Run {
            start,
            length: run,
            power: 0,
        };
        state.count += 1;
        start += run;
        remaining -= run;
    }
    state.merge_force_collapse(items, scratch)
}

impl<T: Copy> MergeState<'_, T> {
    /// Returns the next `minrun`: the list's length divided by `2**shift`, its remainders
    /// carried to the next run.
    fn minrun_next(&mut self) -> usize {
        self.current += self.length;
        let result = self.current >> self.shift;
        self.current &= self.mask;
        result
    }

    /// Merges waiting runs of a greater power than the run before one of `length` items.
    fn found_new_run(
        &mut self,
        items: &mut [T],
        scratch: &mut [T],
        length: usize,
    ) -> Result<(), Raised> {
        if self.count == 0 {
            return Ok(());
        }
        let last = self.runs[self.count - 1];
        let power = power_of(last.start, last.length, length, self.length);
        while self.count > 1 && self.runs[self.count - 2].power > power {
            self.merge_at(items, scratch, self.count - 2)?;
        }
        self.runs[self.count - 1].power = power;
        Ok(())
    }

    /// Merges every waiting run, the shorter neighbour of the last first.
    fn merge_force_collapse(&mut self, items: &mut [T], scratch: &mut [T]) -> Result<(), Raised> {
        while self.count > 1 {
            let mut at = self.count - 2;
            if at > 0 && self.runs[at - 1].length < self.runs[at + 1].length {
                at -= 1;
            }
            self.merge_at(items, scratch, at)?;
        }
        Ok(())
    }

    /// Merges the runs at `at` and `at + 1` of the waiting runs.
    fn merge_at(&mut self, items: &mut [T], scratch: &mut [T], at: usize) -> Result<(), Raised> {
        let Run {
            start: mut first,
            length: mut first_length,
            ..
        } = self.runs[at];
        let Run {
            start: second,
            length: second_length,
            ..
        } = self.runs[at + 1];
        self.runs[at].length = first_length + second_length;
        if at + 3 == self.count {
            self.runs[at + 1] = self.runs[at + 2];
        }
        self.count -= 1;
        let less = self.less;
        // Items of the first run before where the second's first item goes are in place.
        let skipped = gallop_right(items[second], &items[first..first + first_length], 0, less)?;
        first += skipped;
        first_length -= skipped;
        if first_length == 0 {
            return Ok(());
        }
        // Items of the second run after where the first's last item goes are in place.
        let kept = gallop_left(
            items[first + first_length - 1],
            &items[second..second + second_length],
            second_length - 1,
            less,
        )?;
        if kept == 0 {
            return Ok(());
        }
        if first_length <= kept {
            self.merge_low(items, scratch, (first, first_length), kept)
        } else {
            self.merge_high(items, scratch, (first, first_length), kept)
        }
    }

    /// Merges the run of `count` items at `start` with the `others` items that follow it,
    /// copying the first run aside; the first run is the shorter. CPython's `merge_lo`.
    #[allow(
        clippy::too_many_lines,
        reason = "one function, as CPython's, so the order of its comparisons can be checked against it"
    )]
    fn merge_low(
        &mut self,
        items: &mut [T],
        scratch: &mut [T],
        (start, mut count): (usize, usize),
        mut others: usize,
    ) -> Result<(), Raised> {
        let less = self.less;
        scratch[..count].copy_from_slice(&items[start..start + count]);
        let (mut destination, mut a, mut b) = (start, 0, start + count);
        items[destination] = items[b];
        destination += 1;
        b += 1;
        others -= 1;
        let mut result = Ok(());
        'merge: {
            if others == 0 {
                break 'merge;
            }
            if count == 1 {
                // The first run's one item goes after the rest of the second.
                items.copy_within(b..b + others, destination);
                items[destination + others] = scratch[a];
                return Ok(());
            }
            let mut min_gallop = self.min_gallop;
            loop {
                let (mut a_wins, mut b_wins) = (0, 0);
                loop {
                    match less(items[b], scratch[a]) {
                        Err(raised) => {
                            result = Err(raised);
                            break 'merge;
                        }
                        Ok(true) => {
                            items[destination] = items[b];
                            destination += 1;
                            b += 1;
                            b_wins += 1;
                            a_wins = 0;
                            others -= 1;
                            if others == 0 {
                                break 'merge;
                            }
                            if b_wins >= min_gallop {
                                break;
                            }
                        }
                        Ok(false) => {
                            items[destination] = scratch[a];
                            destination += 1;
                            a += 1;
                            a_wins += 1;
                            b_wins = 0;
                            count -= 1;
                            if count == 1 {
                                items.copy_within(b..b + others, destination);
                                items[destination + others] = scratch[a];
                                return Ok(());
                            }
                            if a_wins >= min_gallop {
                                break;
                            }
                        }
                    }
                }
                min_gallop += 1;
                loop {
                    min_gallop -= usize::from(min_gallop > 1);
                    self.min_gallop = min_gallop;
                    let found = match gallop_right(items[b], &scratch[a..a + count], 0, less) {
                        Ok(found) => found,
                        Err(raised) => {
                            result = Err(raised);
                            break 'merge;
                        }
                    };
                    a_wins = found;
                    if found > 0 {
                        items[destination..destination + found]
                            .copy_from_slice(&scratch[a..a + found]);
                        destination += found;
                        a += found;
                        count -= found;
                        if count == 1 {
                            items.copy_within(b..b + others, destination);
                            items[destination + others] = scratch[a];
                            return Ok(());
                        }
                        // Only an inconsistent comparison empties the run here.
                        if count == 0 {
                            break 'merge;
                        }
                    }
                    items[destination] = items[b];
                    destination += 1;
                    b += 1;
                    others -= 1;
                    if others == 0 {
                        break 'merge;
                    }
                    let found = match gallop_left(scratch[a], &items[b..b + others], 0, less) {
                        Ok(found) => found,
                        Err(raised) => {
                            result = Err(raised);
                            break 'merge;
                        }
                    };
                    b_wins = found;
                    if found > 0 {
                        items.copy_within(b..b + found, destination);
                        destination += found;
                        b += found;
                        others -= found;
                        if others == 0 {
                            break 'merge;
                        }
                    }
                    items[destination] = scratch[a];
                    destination += 1;
                    a += 1;
                    count -= 1;
                    if count == 1 {
                        items.copy_within(b..b + others, destination);
                        items[destination + others] = scratch[a];
                        return Ok(());
                    }
                    if a_wins < MIN_GALLOP && b_wins < MIN_GALLOP {
                        break;
                    }
                }
                // Leaving galloping costs: it starts later next time.
                min_gallop += 1;
                self.min_gallop = min_gallop;
            }
        }
        // What is left of the first run goes after what was merged.
        items[destination..destination + count].copy_from_slice(&scratch[a..a + count]);
        result
    }

    /// Merges the run of `count` items at `start` with the `others` items that follow it,
    /// from the end, copying the second run aside; the second run is the shorter. CPython's
    /// `merge_hi`. Positions are signed, since one may pass the start of its run by one.
    #[allow(
        clippy::too_many_lines,
        reason = "one function, as CPython's, so the order of its comparisons can be checked against it"
    )]
    #[allow(
        clippy::cast_possible_wrap,
        clippy::cast_sign_loss,
        reason = "a list's positions fit in an isize, and a position is read only when it is not negative"
    )]
    fn merge_high(
        &mut self,
        items: &mut [T],
        scratch: &mut [T],
        (start, mut count): (usize, usize),
        mut others: usize,
    ) -> Result<(), Raised> {
        let less = self.less;
        let base = start as isize;
        scratch[..others].copy_from_slice(&items[start + count..start + count + others]);
        let at = |position: isize| position as usize;
        let mut destination = base + (count + others) as isize - 1;
        let mut a = base + count as isize - 1;
        let mut b = others as isize - 1;
        items[at(destination)] = items[at(a)];
        destination -= 1;
        a -= 1;
        count -= 1;
        let mut result = Ok(());
        // Puts the second run's first item before what is left of the first run.
        let copy_a = |items: &mut [T], destination: isize, a: isize, count: usize, b: isize| {
            let count = count as isize;
            items.copy_within(at(a + 1 - count)..at(a + 1), at(destination + 1 - count));
            items[at(destination - count)] = scratch[at(b)];
        };
        'merge: {
            if count == 0 {
                break 'merge;
            }
            if others == 1 {
                copy_a(items, destination, a, count, b);
                return Ok(());
            }
            let mut min_gallop = self.min_gallop;
            loop {
                let (mut a_wins, mut b_wins) = (0, 0);
                loop {
                    match less(scratch[at(b)], items[at(a)]) {
                        Err(raised) => {
                            result = Err(raised);
                            break 'merge;
                        }
                        Ok(true) => {
                            items[at(destination)] = items[at(a)];
                            destination -= 1;
                            a -= 1;
                            a_wins += 1;
                            b_wins = 0;
                            count -= 1;
                            if count == 0 {
                                break 'merge;
                            }
                            if a_wins >= min_gallop {
                                break;
                            }
                        }
                        Ok(false) => {
                            items[at(destination)] = scratch[at(b)];
                            destination -= 1;
                            b -= 1;
                            b_wins += 1;
                            a_wins = 0;
                            others -= 1;
                            if others == 1 {
                                copy_a(items, destination, a, count, b);
                                return Ok(());
                            }
                            if b_wins >= min_gallop {
                                break;
                            }
                        }
                    }
                }
                min_gallop += 1;
                loop {
                    min_gallop -= usize::from(min_gallop > 1);
                    self.min_gallop = min_gallop;
                    let first_run = &items[start..start + count];
                    let found = match gallop_right(scratch[at(b)], first_run, count - 1, less) {
                        Ok(found) => count - found,
                        Err(raised) => {
                            result = Err(raised);
                            break 'merge;
                        }
                    };
                    a_wins = found;
                    if found > 0 {
                        let moved = found as isize;
                        destination -= moved;
                        a -= moved;
                        items.copy_within(at(a + 1)..at(a + 1 + moved), at(destination + 1));
                        count -= found;
                        if count == 0 {
                            break 'merge;
                        }
                    }
                    items[at(destination)] = scratch[at(b)];
                    destination -= 1;
                    b -= 1;
                    others -= 1;
                    if others == 1 {
                        copy_a(items, destination, a, count, b);
                        return Ok(());
                    }
                    let found =
                        match gallop_left(items[at(a)], &scratch[..others], others - 1, less) {
                            Ok(found) => others - found,
                            Err(raised) => {
                                result = Err(raised);
                                break 'merge;
                            }
                        };
                    b_wins = found;
                    if found > 0 {
                        let moved = found as isize;
                        destination -= moved;
                        b -= moved;
                        items[at(destination + 1)..at(destination + 1 + moved)]
                            .copy_from_slice(&scratch[at(b + 1)..at(b + 1 + moved)]);
                        others -= found;
                        if others == 1 {
                            copy_a(items, destination, a, count, b);
                            return Ok(());
                        }
                        // Only an inconsistent comparison empties the run here.
                        if others == 0 {
                            break 'merge;
                        }
                    }
                    items[at(destination)] = items[at(a)];
                    destination -= 1;
                    a -= 1;
                    count -= 1;
                    if count == 0 {
                        break 'merge;
                    }
                    if a_wins < MIN_GALLOP && b_wins < MIN_GALLOP {
                        break;
                    }
                }
                min_gallop += 1;
                self.min_gallop = min_gallop;
            }
        }
        // What is left of the second run goes before what was merged.
        if others > 0 {
            let first = destination + 1 - others as isize;
            items[at(first)..at(first) + others].copy_from_slice(&scratch[..others]);
        }
        result
    }
}

/// Returns the power of the run of `first_length` items at `first`, followed by a run of
/// `second_length`, in a list of `length` items: the first bit where the midpoints of the two
/// runs, as fractions of the length, differ. CPython's `powerloop`.
fn power_of(first: usize, first_length: usize, second_length: usize, length: usize) -> u32 {
    let mut a = 2 * first + first_length;
    let mut b = a + first_length + second_length;
    let mut power = 0;
    loop {
        power += 1;
        if a >= length {
            a -= length;
            b -= length;
        } else if b >= length {
            break;
        }
        a <<= 1;
        b <<= 1;
    }
    power
}

/// Returns the length of the run at the start of `items`, at most `remaining` items long,
/// made ascending in place: a descending run is reversed, keeping equal items in order, and an
/// ascending suffix after it joins it. CPython's `count_run`.
fn count_run<T: Copy>(
    items: &mut [T],
    remaining: usize,
    less: Less<'_, T>,
) -> Result<usize, Raised> {
    let mut length = 1;
    while length < remaining {
        if less(items[length], items[length - 1])? {
            break;
        }
        length += 1;
    }
    if length == remaining {
        return Ok(length);
    }
    // An ascending run of more than one item is a run of equal items when its first is not
    // less than its last, and so the start of a descending run.
    if length > 1 {
        if less(items[0], items[length - 1])? {
            return Ok(length);
        }
        items[..length].reverse();
    }
    length += 1;
    // Equal items of the descending run are reversed as they end, so the reversal of the
    // whole run puts them back in their order.
    let mut equal = 0;
    let reverse_equal = |items: &mut [T], length: usize, equal: &mut usize| {
        if *equal > 0 {
            *equal += 1;
            items[length - *equal..length].reverse();
            *equal = 0;
        }
    };
    while length < remaining {
        if less(items[length], items[length - 1])? {
            reverse_equal(items, length, &mut equal);
        } else if less(items[length - 1], items[length])? {
            break;
        } else {
            equal += 1;
        }
        length += 1;
    }
    reverse_equal(items, length, &mut equal);
    items[..length].reverse();
    while length < remaining {
        if less(items[length], items[length - 1])? {
            break;
        }
        length += 1;
    }
    Ok(length)
}

/// Sorts `items` by binary insertion, its first `sorted` items already in order. CPython's
/// `binarysort`.
fn binary_sort<T: Copy>(items: &mut [T], sorted: usize, less: Less<'_, T>) -> Result<(), Raised> {
    for next in sorted.max(1)..items.len() {
        let pivot = items[next];
        let (mut low, mut high) = (0, next);
        while low < high {
            let middle = usize::midpoint(low, high);
            if less(pivot, items[middle])? {
                high = middle;
            } else {
                low = middle + 1;
            }
        }
        items.copy_within(low..next, low + 1);
        items[low] = pivot;
    }
    Ok(())
}

/// Returns where `key` goes in the sorted `run`, before any equal item, searching out from
/// `hint`. CPython's `gallop_left`.
#[allow(
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    reason = "a run's positions fit in an isize, and the search reads only positions inside it"
)]
fn gallop_left<T: Copy>(
    key: T,
    run: &[T],
    hint: usize,
    less: Less<'_, T>,
) -> Result<usize, Raised> {
    let (length, hint) = (run.len() as isize, hint as isize);
    let at = |offset: isize| run[offset as usize];
    let (mut last, mut offset) = (0_isize, 1_isize);
    if less(at(hint), key)? {
        // Gallop right until run[hint + last] < key <= run[hint + offset].
        let most = length - hint;
        while offset < most {
            if less(at(hint + offset), key)? {
                last = offset;
                offset = (offset << 1) + 1;
            } else {
                break;
            }
        }
        offset = offset.min(most);
        last += hint;
        offset += hint;
    } else {
        // Gallop left until run[hint - offset] < key <= run[hint - last].
        let most = hint + 1;
        while offset < most {
            if less(at(hint - offset), key)? {
                break;
            }
            last = offset;
            offset = (offset << 1) + 1;
        }
        offset = offset.min(most);
        (last, offset) = (hint - offset, hint - last);
    }
    last += 1;
    while last < offset {
        let middle = last + ((offset - last) >> 1);
        if less(at(middle), key)? {
            last = middle + 1;
        } else {
            offset = middle;
        }
    }
    Ok(offset as usize)
}

/// Returns where `key` goes in the sorted `run`, after any equal item, searching out from
/// `hint`. CPython's `gallop_right`.
#[allow(
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    reason = "a run's positions fit in an isize, and the search reads only positions inside it"
)]
fn gallop_right<T: Copy>(
    key: T,
    run: &[T],
    hint: usize,
    less: Less<'_, T>,
) -> Result<usize, Raised> {
    let (length, hint) = (run.len() as isize, hint as isize);
    let at = |offset: isize| run[offset as usize];
    let (mut last, mut offset) = (0_isize, 1_isize);
    if less(key, at(hint))? {
        // Gallop left until run[hint - offset] <= key < run[hint - last].
        let most = hint + 1;
        while offset < most {
            if less(key, at(hint - offset))? {
                last = offset;
                offset = (offset << 1) + 1;
            } else {
                break;
            }
        }
        offset = offset.min(most);
        (last, offset) = (hint - offset, hint - last);
    } else {
        // Gallop right until run[hint + last] <= key < run[hint + offset].
        let most = length - hint;
        while offset < most {
            if less(key, at(hint + offset))? {
                break;
            }
            last = offset;
            offset = (offset << 1) + 1;
        }
        offset = offset.min(most);
        last += hint;
        offset += hint;
    }
    last += 1;
    while last < offset {
        let middle = last + ((offset - last) >> 1);
        if less(key, at(middle))? {
            offset = middle;
        } else {
            last = middle + 1;
        }
    }
    Ok(offset as usize)
}

#[cfg(test)]
mod tests;
