package objectiveai

import (
	"reflect"

	"github.com/go-playground/validator/v10"
	orderedmap "github.com/wk8/go-ordered-map/v2"
)

// ToMapAny converts the OrderedMap to a map[any]any for validator dive support.
// This avoids reflection on unexported fields of the underlying ordered map.
func (om OrderedMap[K, V]) ToMapAny() map[any]any {
	if om.inner == nil {
		return nil
	}
	result := make(map[any]any, om.inner.Len())
	for pair := om.inner.Oldest(); pair != nil; pair = pair.Next() {
		result[pair.Key] = pair.Value
	}
	return result
}

// orderedMapToMap converts any OrderedMap[K, V] to map[any]any via the ToMapAny method.
func orderedMapToMap(field reflect.Value) interface{} {
	m := field.MethodByName("ToMapAny")
	if !m.IsValid() {
		return nil
	}
	return m.Call(nil)[0].Interface()
}

// RegisterOrderedMapTypes registers OrderedMap instantiations with the validator
// so that "dive" tags work. Call once per concrete OrderedMap[K, V] type.
func RegisterOrderedMapTypes(v *validator.Validate, types ...interface{}) {
	v.RegisterCustomTypeFunc(orderedMapToMap, types...)
}

// OrderedMap is a JSON-order-preserving map. It wraps orderedmap.OrderedMap
// in a value-type struct so that:
//   - Value fields marshal correctly (no pointer-receiver issue)
//   - *OrderedMap[K, V] expresses nullability naturally
type OrderedMap[K comparable, V any] struct {
	inner *orderedmap.OrderedMap[K, V]
}

func NewOrderedMap[K comparable, V any](pairs ...orderedmap.Pair[K, V]) OrderedMap[K, V] {
	return OrderedMap[K, V]{inner: orderedmap.New[K, V](orderedmap.WithInitialData(pairs...))}
}

func (om OrderedMap[K, V]) Set(key K, value V) {
	if om.inner == nil {
		return
	}
	om.inner.Set(key, value)
}

func (om OrderedMap[K, V]) Get(key K) (V, bool) {
	if om.inner == nil {
		var zero V
		return zero, false
	}
	return om.inner.Get(key)
}

func (om OrderedMap[K, V]) Len() int {
	if om.inner == nil {
		return 0
	}
	return om.inner.Len()
}

func (om OrderedMap[K, V]) Oldest() *orderedmap.Pair[K, V] {
	if om.inner == nil {
		return nil
	}
	return om.inner.Oldest()
}

func (om OrderedMap[K, V]) Newest() *orderedmap.Pair[K, V] {
	if om.inner == nil {
		return nil
	}
	return om.inner.Newest()
}

func (om OrderedMap[K, V]) Delete(key K) (V, bool) {
	if om.inner == nil {
		var zero V
		return zero, false
	}
	return om.inner.Delete(key)
}

func (om OrderedMap[K, V]) MarshalJSON() ([]byte, error) {
	if om.inner == nil {
		return []byte("{}"), nil
	}
	return om.inner.MarshalJSON()
}

func (om *OrderedMap[K, V]) UnmarshalJSON(data []byte) error {
	if om.inner == nil {
		om.inner = orderedmap.New[K, V]()
	}
	return om.inner.UnmarshalJSON(data)
}
