<?php
namespace App;

function add($a, $b) {
    return $a + $b;
}

const PI = 3.14159;

class Circle {
    public $radius;
    public function __construct($r) {
        $this->radius = $r;
    }
    public function area() {
        return PI * $this->radius * $this->radius;
    }
}

$circle = new Circle(5);
echo add(2, 3);
?>
