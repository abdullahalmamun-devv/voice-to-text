import unittest
from benchmark import edits,norm,percentile95

class MetricsTests(unittest.TestCase):
    def test_insertions_can_exceed_one_hundred_percent(self):
        self.assertEqual(edits(['a'],['b','c','d']),3)
    def test_negation_repetition_and_digits_preserved(self):
        self.assertEqual(norm('না না, ১২৩ টাকা!'),'না না ১২৩ টাকা')
        self.assertNotEqual(norm('১২৩'),norm('123'))
    def test_nfc_and_joiners(self):
        self.assertEqual(norm('cafe\u0301'),'café')
        self.assertIn('\u200d',norm('ক\u200dষ'))
    def test_nearest_rank_p95(self):
        self.assertEqual(percentile95(list(range(1,21))),19)
    def test_empty_scores_and_identical_text(self):
        self.assertIsNone(percentile95([]))
        self.assertEqual(edits('বাংলা','বাংলা'),0)

if __name__=='__main__':unittest.main()
