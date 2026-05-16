package objectiveai

// pushOptionUint64 adds other to self. If self is nil, adopts other's value.
func pushOptionUint64(self *uint64, other *uint64) *uint64 {
	if self != nil && other != nil {
		sum := *self + *other
		return &sum
	}
	if other != nil {
		v := *other
		return &v
	}
	return self
}

// pushOptionFloat64 adds other to self. If self is nil, adopts other's value.
func pushOptionFloat64(self *float64, other *float64) *float64 {
	if self != nil && other != nil {
		sum := *self + *other
		return &sum
	}
	if other != nil {
		v := *other
		return &v
	}
	return self
}

// pushOptionString concatenates other onto self. If self is nil, adopts other.
func pushOptionString(self *string, other *string) *string {
	if self != nil && other != nil {
		s := *self + *other
		return &s
	}
	if other != nil {
		s := *other
		return &s
	}
	return self
}

// pushByIndex merges items from other into self by matching an index field.
// The getIndex function extracts the index, and push merges matching items.
func pushByIndex[T any](self *[]T, other []T, getIndex func(*T) uint64, push func(*T, *T)) {
	indexMap := make(map[uint64]int)
	for i := range *self {
		idx := getIndex(&(*self)[i])
		indexMap[idx] = i
	}
	for i := range other {
		idx := getIndex(&other[i])
		if pos, ok := indexMap[idx]; ok {
			push(&(*self)[pos], &other[i])
		} else {
			*self = append(*self, other[i])
			indexMap[idx] = len(*self) - 1
		}
	}
}

// pushByNullableIndex is like pushByIndex but the index can be nil.
// Items with nil index are always appended (never matched).
func pushByNullableIndex[T any](self *[]T, other []T, getIndex func(*T) *uint64, push func(*T, *T)) {
	indexMap := make(map[uint64]int)
	for i := range *self {
		if idx := getIndex(&(*self)[i]); idx != nil {
			indexMap[*idx] = i
		}
	}
	for i := range other {
		idx := getIndex(&other[i])
		if idx != nil {
			if pos, ok := indexMap[*idx]; ok {
				push(&(*self)[pos], &other[i])
			} else {
				*self = append(*self, other[i])
				indexMap[*idx] = len(*self) - 1
			}
		} else {
			*self = append(*self, other[i])
		}
	}
}

// pushLazySetTrue only sets to true, never overwrites to false.
func pushLazySetTrue(self *bool, other *bool) *bool {
	if other != nil && *other {
		v := true
		return &v
	}
	return self
}

// pushReplace replaces self with other if other is not nil (latest wins).
func pushReplace[T any](self *T, other *T) *T {
	if other != nil {
		return other
	}
	return self
}
