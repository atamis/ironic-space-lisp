(def assoc
     (lambda (id lst)
       (if (empty? lst)
           'none
           (let (pair (car lst)
                      key (car pair))
             (if (= key id)
                 (car (cdr pair))
                 (assoc id (cdr lst)))))))

(def foldl
     (lambda (f init lst)
       (if (empty? lst)
           init
           (foldl f (f (car lst) init) (cdr lst)))))

(def map
     (lambda (f lst)
       (if (empty? lst)
           '()
           (cons (f (car lst)) (map f (cdr lst))))))

(def eval
     (lambda (expr env)
       (if (list? expr)
           (let (name (car expr)
                      r (cdr expr))
             (if (= name '+)
                 (foldl + 0 (map (lambda (e) (eval e env)) r))
                 'not-implemented))
             (if (keyword? expr)
                 (assoc expr env)
                 expr))))


(eval '(+ 1 2 3) '((test 1)))
