#include "config.h"
#ifdef __ESP
#ifndef INCLUDE_FREERTOS
#define INCLUDE_FREERTOS
#include "freertos/FreeRTOS.h"
#endif
#endif
#ifdef __ARDUINO
#include <Arduino.h>
#endif

#define true 1
#define false 0
typedef union value_t
{
    int32_t num;
    uint32_t obj_header;
    uint8_t *ip;
    union value_t *bp;
    union value_t *obj;
} value_t;
typedef void (*dev_input_t)(value_t *);
typedef struct call_frame
{
    uint8_t *next_ip;
    value_t *bp, *sp;
    struct call_frame *caller;
} call_frame;
typedef enum
{
    EMFRP_OK,
    EMFRP_RUNTIME_ERR,
    EMFRP_PANIC,
    EMFRP_TODO,
    EMFRP_OUTOF_MEMORY

} emfrp_result_t;
typedef union
{
    uint8_t *insns;
    dev_input_t dev;
} upd_action_t;

typedef void (*output_action_t)(value_t *);
typedef value_t node_v_t;
typedef value_t data_t;
typedef uint8_t *func_t;
typedef struct obj_list
{
    value_t obj;
    struct obj_list *next;
} obj_list;
typedef struct
{
    data_t *lst;
    int len;
    int cap;
} data_list_t;

typedef struct
{
    func_t *lst;
    int len;
    int cap;
} func_list_t;

typedef struct
{
    value_t *values;
    upd_action_t *action;
    int len;
    int cap;
} node_list_t;
typedef struct
{
    value_t *v_stack, *node_last;
    uint8_t *update;
    data_list_t data_list;
    func_list_t func_list;
    node_list_t node_list;
    output_action_t *output_actions;
    int output_nd_len;
} emfrp_machine_t;

typedef enum
{
    BC_None = 1,
    BC_Nil = 2,
    BC_Not = 3,
    BC_Minus = 4,
    BC_Add = 5,
    BC_Sub = 6,
    BC_Mul = 7,
    BC_Div = 8,
    BC_Mod = 9,
    BC_ShiftL = 10,
    BC_ShiftR = 11,
    BC_Ls = 12,
    BC_Leq = 13,
    BC_Gt = 14,
    BC_Geq = 15,
    BC_Eq = 16,
    BC_Neq = 17,
    BC_BitAnd = 18,
    BC_BitOr = 19,
    BC_BitXor = 20,
    BC_Return = 21,
    BC_Print = 22,
    BC_PRINTOBJ = 23,
    BC_Halt = 24,
    BC_Peek = 25,
    BC_PushTrue = 26,
    BC_PushFalse = 27,
    BC_ABORT = 28,
    BC_INT_0 = 30,
    BC_INT_1 = 31,
    BC_INT_2 = 32,
    BC_INT_3 = 33,
    BC_INT_4 = 34,
    BC_INT_5 = 35,
    BC_INT_6 = 36,
    BC_INT_I8 = 37,
    BC_INT_I16 = 38,
    BC_INT_I32 = 39,
    BC_GET_LOCAL_0 = 40,
    BC_GET_LOCAL_1 = 41,
    BC_GET_LOCAL_2 = 42,
    BC_GET_LOCAL_3 = 43,
    BC_GET_LOCAL_4 = 44,
    BC_GET_LOCAL_5 = 45,
    BC_GET_LOCAL_6 = 46,
    BC_GET_LOCAL_I8 = 47,
    BC_GET_LOCAL_I16 = 48,
    BC_GET_LOCAL_I32 = 49,
    BC_SET_LOCAL_0 = 50,
    BC_SET_LOCAL_1 = 51,
    BC_SET_LOCAL_2 = 52,
    BC_SET_LOCAL_3 = 53,
    BC_SET_LOCAL_4 = 54,
    BC_SET_LOCAL_5 = 55,
    BC_SET_LOCAL_6 = 56,
    BC_SET_LOCAL_I8 = 57,
    BC_SET_LOCAL_I16 = 58,
    BC_SET_LOCAL_I32 = 59,
    BC_ALLOC_LOCAL_1 = 61,
    BC_ALLOC_LOCAL_2 = 62,
    BC_ALLOC_LOCAL_3 = 63,
    BC_ALLOC_LOCAL_4 = 64,
    BC_ALLOC_LOCAL_5 = 65,
    BC_ALLOC_LOCAL_6 = 66,
    BC_ALLOC_LOCAL_U8 = 67,
    BC_ALLOC_LOCAL_U16 = 68,
    BC_ALLOC_LOCAL_U32 = 69,
    BC_POP_1 = 71,
    BC_POP_2 = 72,
    BC_POP_3 = 73,
    BC_POP_4 = 74,
    BC_POP_5 = 75,
    BC_POP_6 = 76,
    BC_POP_U8 = 77,
    BC_POP_U16 = 78,
    BC_POP_U32 = 79,

    BC_Jne8 = 80,
    BC_Jne16 = 81,
    BC_Jne32 = 82,
    BC_Je8 = 83,
    BC_Je16 = 84,
    BC_Je32 = 85,
    BC_J8 = 86,
    BC_J16 = 87,
    BC_J32 = 88,

    BC_GET_LAST_0 = 90,
    BC_GET_LAST_1 = 91,
    BC_GET_LAST_2 = 92,
    BC_GET_LAST_3 = 93,
    BC_GET_LAST_U8 = 94,
    BC_GET_LAST_U16 = 95,
    BC_GET_LAST_U32 = 96,
    BC_SET_NODE_U8 = 97,
    BC_SET_NODE_U16 = 98,
    BC_SET_NODE_U32 = 99,
    BC_OBJ_FIELD_0 = 100,
    BC_OBJ_FIELD_1 = 101,
    BC_OBJ_FIELD_2 = 102,
    BC_OBJ_FIELD_3 = 103,
    BC_OBJ_FIELD_4 = 104,
    BC_OBJ_FIELD_5 = 105,
    BC_OBJ_FIELD_6 = 106,

    BC_UPD_DEV_0 = 110,
    BC_UPD_DEV_1 = 111,
    BC_UPD_DEV_2 = 112,
    BC_UPD_DEV_3 = 113,
    BC_UPD_DEV_U8 = 114,
    BC_UPD_NODE_U8 = 117,
    BC_UPD_NODE_U16 = 118,
    BC_UPD_NODE_U32 = 119,
    BC_O_ACTION_0 = 120,
    BC_O_ACTION_1 = 121,
    BC_O_ACTION_2 = 122,
    BC_O_ACTION_3 = 123,
    BC_O_ACTION_U8 = 124,
    BC_CALL_U8 = 127,
    BC_CALL_U16 = 128,
    BC_CALL_U32 = 129,
    BC_GET_DATA_U8 = 130,
    BC_GET_DATA_U16 = 131,
    BC_GET_DATA_U32 = 132,
    BC_GET_NODE_U8 = 133,
    BC_GET_NODE_U16 = 134,
    BC_GET_NODE_U32 = 135,
    BC_SET_DATA_U8 = 141,
    BC_SET_DATA_U16 = 142,
    BC_SET_DATA_U32 = 143,
    BC_OBJ_TAG = 144,
    BC_SET_LAST_0 = 150,
    BC_SET_LAST_1 = 151,
    BC_SET_LAST_2 = 152,
    BC_SET_LAST_3 = 153,
    BC_SET_LAST_U8 = 154,
    BC_SET_LAST_U16 = 155,
    BC_SET_LAST_U32 = 156,
    BC_END_UPD_U8 = 157,
    BC_END_UPD_U16 = 158,
    BC_END_UPD_U32 = 159,
    BC_ALLOC_OBJ_0 = 160, // max entry
    BC_ALLOC_OBJ_1 = 161,
    BC_ALLOC_OBJ_2 = 162,
    BC_ALLOC_OBJ_3 = 163,
    BC_ALLOC_OBJ_4 = 164,
    BC_ALLOC_OBJ_5 = 165,
    BC_ALLOC_OBJ_6 = 166,
    BC_ALLOC_OBJ_U8 = 167,
    BC_DROP_LOCAL_OBJ_0 = 170,
    BC_DROP_LOCAL_OBJ_1 = 171,
    BC_DROP_LOCAL_OBJ_2 = 172,
    BC_DROP_LOCAL_OBJ_3 = 173,
    BC_DROP_LOCAL_OBJ_4 = 174,
    BC_DROP_LOCAL_OBJ_5 = 175,
    BC_DROP_LOCAL_OBJ_6 = 176,
    BC_DROP_LOCAL_OBJ_I8 = 177,
    BC_DROP_LOCAL_OBJ_I16 = 178,
    BC_DROP_LOCAL_OBJ_I32 = 179,
    BC_GET_LOCAL_REF_0 = 180,
    BC_GET_LOCAL_REF_1 = 181,
    BC_GET_LOCAL_REF_2 = 182,
    BC_GET_LOCAL_REF_3 = 183,
    BC_GET_LOCAL_REF_4 = 184,
    BC_GET_LOCAL_REF_5 = 185,
    BC_GET_LOCAL_REF_6 = 186,
    BC_GET_LOCAL_REF_I8 = 187,
    BC_GET_LOCAL_REF_I16 = 188,
    BC_GET_LOCAL_REF_I32 = 189,
    BC_SET_LOCAL_REF_0 = 190,
    BC_SET_LOCAL_REF_1 = 191,
    BC_SET_LOCAL_REF_2 = 192,
    BC_SET_LOCAL_REF_3 = 193,
    BC_SET_LOCAL_REF_4 = 194,
    BC_SET_LOCAL_REF_5 = 195,
    BC_SET_LOCAL_REF_6 = 196,
    BC_SET_LOCAL_REF_I8 = 197,
    BC_SET_LOCAL_REF_I16 = 198,
    BC_SET_LOCAL_REF_I32 = 199,
    BC_OBJ_FIELD_REF_0 = 200,
    BC_OBJ_FIELD_REF_1 = 201,
    BC_OBJ_FIELD_REF_2 = 202,
    BC_OBJ_FIELD_REF_3 = 203,
    BC_OBJ_FIELD_REF_4 = 204,
    BC_OBJ_FIELD_REF_5 = 205,
    BC_OBJ_FIELD_REF_6 = 206,
    BC_END_UPD_OBJ_U8 = 210,
    BC_END_UPD_OBJ_U16 = 211,
    BC_END_UPD_OBJ_U32 = 212,
    BC_GET_NODE_REF_U8 = 213,
    BC_GET_NODE_REF_U16 = 214,
    BC_GET_NODE_REF_U32 = 215,
    BC_GET_DATA_REF_U8 = 216,
    BC_GET_DATA_REF_U16 = 217,
    BC_GET_DATA_REF_U32 = 218,
    BC_GET_LAST_REF_0 = 220,
    BC_GET_LAST_REF_1 = 221,
    BC_GET_LAST_REF_2 = 222,
    BC_GET_LAST_REF_3 = 223,
    BC_GET_LAST_REF_U8 = 224,
    BC_GET_LAST_REF_U16 = 225,
    BC_GET_LAST_REF_U32 = 226,
    BC_SET_DATA_REF_U8 = 227,
    BC_SET_DATA_REF_U16 = 228,
    BC_SET_DATA_REF_U32 = 229,
    BC_SET_LAST_REF_0 = 230,
    BC_SET_LAST_REF_1 = 231,
    BC_SET_LAST_REF_2 = 232,
    BC_SET_LAST_REF_3 = 233,
    BC_SET_LAST_REF_U8 = 234,
    BC_SET_LAST_REF_U16 = 235,
    BC_SET_LAST_REF_U32 = 236,
    BC_SET_NODE_REF_U8 = 237,
    BC_SET_NODE_REF_U16 = 238,
    BC_SET_NODE_REF_U32 = 239,
    BC_DROP_LAST_U8 = 240,
    BC_DROP_LAST_U16 = 241,
    BC_DROP_LAST_U32 = 242,
    BC_J0 = 243,
    BC_J1 = 244,
    BC_Je0 = 245,
    BC_Je1 = 246,
    BC_Jne0 = 247,
    BC_Jne1 = 248,

} bytecode;
emfrp_result_t emfrp_init(emfrp_machine_t *em, int n_input_node, int n_output_node);
void emfrp_add_input_node(emfrp_machine_t *em, value_t init, dev_input_t driver);
void emfrp_add_output_node(emfrp_machine_t *em, value_t init, output_action_t driver);
emfrp_result_t emfrp_update(emfrp_machine_t *em);
emfrp_result_t emfrp_new_bytecode(emfrp_machine_t *em, int data_len, uint8_t *data);
value_t emfrp_int(int32_t i);
value_t emfrp_true();
value_t emfrp_false();
#ifdef EMFRP_MEASURE_HEAP
#endif
