use gix_imara_diff::{Algorithm, Diff, InternedInput};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineMapping {
    /// For each line index in the worktree file (0-indexed),
    /// contains `Some(head_line_idx)` if the line is unmodified from HEAD,
    /// or `None` if added or modified in the worktree.
    pub worktree_to_head: Vec<Option<usize>>,
}

pub fn map_worktree_to_head(head_bytes: &[u8], worktree_bytes: &[u8]) -> LineMapping {
    let input = InternedInput::new(head_bytes, worktree_bytes);
    let diff = Diff::compute(Algorithm::Histogram, &input);

    let after_len = input.after.len();
    let before_len = input.before.len();
    let mut worktree_to_head = Vec::with_capacity(after_len);

    let mut prev_before = 0usize;
    let mut prev_after = 0usize;

    for hunk in diff.hunks() {
        let after_start = hunk.after.start as usize;

        // Unchanged lines between previous change and this change
        let unchanged_len = after_start.saturating_sub(prev_after);
        for i in 0..unchanged_len {
            let before_idx = prev_before + i;
            if before_idx < before_len {
                worktree_to_head.push(Some(before_idx));
            } else {
                worktree_to_head.push(None);
            }
        }

        // Mark lines added or modified in the worktree as None (uncommitted)
        let changed_len = (hunk.after.end as usize).saturating_sub(after_start);
        for _ in 0..changed_len {
            worktree_to_head.push(None);
        }

        prev_before = hunk.before.end as usize;
        prev_after = hunk.after.end as usize;
    }

    // Map any trailing unchanged lines after the last change
    let trailing_len = after_len.saturating_sub(prev_after);
    for i in 0..trailing_len {
        let before_idx = prev_before + i;
        if before_idx < before_len {
            worktree_to_head.push(Some(before_idx));
        } else {
            worktree_to_head.push(None);
        }
    }

    LineMapping { worktree_to_head }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_files_map_one_to_one() {
        let head = b"line 1\nline 2\nline 3\n";
        let worktree = b"line 1\nline 2\nline 3\n";
        let mapping = map_worktree_to_head(head, worktree);
        assert_eq!(mapping.worktree_to_head, vec![Some(0), Some(1), Some(2)]);
    }

    #[test]
    fn unmodified_and_modified_lines() {
        let head = b"a\nb\nc\nd\n";
        let worktree = b"a\nb_mod\nc\nd_new\n";
        let mapping = map_worktree_to_head(head, worktree);
        assert_eq!(mapping.worktree_to_head, vec![Some(0), None, Some(2), None]);
    }

    #[test]
    fn inserted_lines_offset_subsequent_head_lines() {
        let head = b"a\nc\n";
        let worktree = b"a\nb\nc\n";
        let mapping = map_worktree_to_head(head, worktree);
        assert_eq!(mapping.worktree_to_head, vec![Some(0), None, Some(1)]);
    }

    #[test]
    fn deleted_lines_are_omitted() {
        let head = b"a\nb\nc\n";
        let worktree = b"a\nc\n";
        let mapping = map_worktree_to_head(head, worktree);
        assert_eq!(mapping.worktree_to_head, vec![Some(0), Some(2)]);
    }

    #[test]
    fn completely_new_worktree_file() {
        let head = b"";
        let worktree = b"new 1\nnew 2\n";
        let mapping = map_worktree_to_head(head, worktree);
        assert_eq!(mapping.worktree_to_head, vec![None, None]);
    }

    #[test]
    fn emptied_worktree_file() {
        let head = b"old 1\nold 2\n";
        let worktree = b"";
        let mapping = map_worktree_to_head(head, worktree);
        assert!(mapping.worktree_to_head.is_empty());
    }
}
