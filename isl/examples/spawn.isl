(def spawn
  (fn [f]
    (if (fork)
      (do (f) (terminate 'ok))
      #f)))

(def me (pid))

(spawn (fn [] (do (print 'hello) (send me 'hello-from-spawn))))
(print 'hello-from-main)
(print (wait))

'done
