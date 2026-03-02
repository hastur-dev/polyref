import React, { useState, useEffect, NonExistent } from 'react';  // ERROR: NonExistent

// Valid hook usage
const [count, setCount] = useState(0);
const [name, setName] = useState<string>("");

// Invalid: wrong destructuring
const [a, b, c] = useState(0);  // ERROR: useState returns [S, SetStateAction<S>], not 3 values

// Valid effect
useEffect(() => {
    console.log(count);
}, [count]);

// Invalid: calling non-existent function
const ctx = useNonExistent();  // ERROR: useNonExistent not exported from react

// Valid
const ref = React.createRef<HTMLDivElement>();
