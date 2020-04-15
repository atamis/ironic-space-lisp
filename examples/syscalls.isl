(def assert-eq
  (fn (a b)
    (if (= a b)
      nil
      (error (list 'assert-error a b)))))

(assert-eq true true)


(assert-eq (len '(1)) 1)
(assert-eq (len '()) 0)
(assert-eq (len '(1 2 3 4 5)) 5)
(assert-eq (len '(:a :b :c :d :e)) 5)

(assert-eq (cons 1 '()) '(1))
(assert-eq (cons 1 '(2)) '(1 2))

(assert-eq (car '(1)) 1)
(assert-eq (cdr '(1)) '())

(assert-eq (first '(1)) 1)
(assert-eq (rest '(1)) '())

(assert-eq (empty? '()) true)
(assert-eq (empty? '(1)) false)

(assert-eq (nth 0 '(1 2 3)) 1)
(assert-eq (nth 1 '(1 2 3)) 2)
(assert-eq (nth 2 '(1 2 3)) 3)

:done
