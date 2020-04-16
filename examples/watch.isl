(def spawn
  (fn (f)
    (let [orig (pid)]
      (if (fork)
        (do (send orig (pid)) (f) (terminate :ok))
        (wait)))))

(print (list 'main (spawn (fn () (print (list 'spawn (pid)))))))

(let [child (spawn (fn () (print (list 'spawn (wait)))))]
  (watch child)
  (send child 'hello)
  (print (list 'main (wait))))

:done
