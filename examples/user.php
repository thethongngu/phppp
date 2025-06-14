<?php
namespace App\Models;

class User {
    private $name;
    public function __construct($name) {
        $this->name = $name;
    }
    public function getName() {
        return $this->name;
    }
}

function create_user($name) {
    return new User($name);
}

$john = create_user('John');
echo $john->getName();
?>
