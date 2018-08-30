(def mylen
     (lambda (l)
       (if (empty? l)
           0
         + 1 (mylen (cdr l)))))

(def mylen2
     (lambda
       (l n)
       (if (empty? l)
           n
         (mylen2 (cdr l) (+ 1 n)))))

(def lst '(1 2 3 4 5 6 7))

(mylen lst)
(mylen2 lst)
