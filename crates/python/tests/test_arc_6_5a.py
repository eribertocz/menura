"""ARC-6.5a: a `Handle` may be shared between threads."""

import concurrent.futures
import unittest

from support import QUERY, analyst, slice_handle


class Arc65a(unittest.TestCase):
    def test_arc_6_5a_a_handle_is_shared_between_threads(self):
        handle = slice_handle()
        expected = handle.compile(QUERY, analyst()).document
        with concurrent.futures.ThreadPoolExecutor(max_workers=8) as pool:
            documents = list(
                pool.map(lambda _: handle.compile(QUERY, analyst()).document, range(32))
            )
        self.assertEqual(documents, [expected] * 32)


if __name__ == "__main__":
    unittest.main()
